//! State Store - UI State Tree, System/Task State, Persistence & Subscriptions

use common::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

mod backend;
mod storage;
use storage::{PatchEvent, Store};

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<Store>>,
    /// Active watch streams keyed by subscription id.
    subscriptions: Arc<RwLock<HashMap<String, String>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    info!("Starting State Store");

    let state = AppState {
        store: Arc::new(Mutex::new(Store::new())),
        subscriptions: Arc::new(RwLock::new(HashMap::new())),
    };

    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let socket_path = format!("{}/state-store.sock", socket_dir);
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let _ = tokio::fs::remove_file(&socket_path).await;
    let listener = UnixListener::bind(&socket_path)?;
    info!("State Store listening on {}", socket_path);

    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            handle_connection(stream, state).await;
        });
    }
}

async fn handle_connection(stream: tokio::net::UnixStream, state: AppState) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                match handle_line(trimmed, &state, &mut writer).await {
                    Ok(continue_loop) => {
                        if !continue_loop {
                            break;
                        }
                    }
                    Err(e) => {
                        error!("handle error: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }
}

/// Returns `Ok(false)` when the connection should close (watch stream ended).
async fn handle_line(
    line: &str,
    state: &AppState,
    writer: &mut (impl AsyncWriteExt + Unpin),
) -> anyhow::Result<bool> {
    let msg: McpMessage = serde_json::from_str(line)?;

    if matches!(msg.kind, MessageKind::Request) {
        if let Some(method) = &msg.method {
            if method == "state.watch" {
                return handle_watch(msg, state, writer).await;
            }
        }
    }

    let response = match msg.kind {
        MessageKind::Request => {
            if let Some(method) = msg.method {
                handle_request(method, msg.params, &msg.id, state).await
            } else {
                error_response(&msg.id, "E_INVALID_REQUEST", "Missing method")
            }
        }
        _ => error_response(&msg.id, "E_INVALID_REQUEST", "Only requests supported"),
    };

    writer
        .write_all((serde_json::to_string(&response)? + "\n").as_bytes())
        .await?;
    Ok(true)
}

async fn handle_watch(
    msg: McpMessage,
    state: &AppState,
    writer: &mut (impl AsyncWriteExt + Unpin),
) -> anyhow::Result<bool> {
    let params = msg.params.unwrap_or(serde_json::Value::Null);
    let prefix = params
        .get("path_prefix")
        .or_else(|| params.get("path"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let since_revision = params
        .get("since_revision")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let sub_id = Uuid::new_v4().to_string();
    state
        .subscriptions
        .write()
        .await
        .insert(sub_id.clone(), prefix.clone());

    let ack = success_response(
        &msg.id,
        serde_json::json!({
            "subscription_id": sub_id,
            "path_prefix": prefix,
            "since_revision": since_revision,
        }),
    );
    writer
        .write_all((serde_json::to_string(&ack)? + "\n").as_bytes())
        .await?;

    // Replay current state for paths matching prefix with revision > since_revision.
    {
        let store = state.store.lock().await;
        for (path, sv) in store.list_prefix(&prefix) {
            if sv.revision > since_revision {
                let notif = watch_notification(&path, &sv);
                writer
                    .write_all((serde_json::to_string(&notif)? + "\n").as_bytes())
                    .await?;
            }
        }
    }

    let mut rx = state.store.lock().await.subscribe();
    loop {
        match rx.recv().await {
            Ok(event) => {
                if !prefix.is_empty() && !event.path.starts_with(&prefix) {
                    continue;
                }
                if event.revision <= since_revision {
                    continue;
                }
                let notif = watch_notification_event(&event);
                if writer
                    .write_all((serde_json::to_string(&notif)? + "\n").as_bytes())
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            Err(_) => break,
        }
    }
    Ok(false)
}

fn watch_notification(path: &str, sv: &crate::backend::StoredValue) -> McpMessage {
    McpMessage {
        id: Uuid::new_v4(),
        stream_id: 0,
        kind: MessageKind::Notification,
        method: Some("state.patch_event".into()),
        params: Some(serde_json::json!({
            "path": path,
            "new_value": sv.value,
            "revision": sv.revision,
        })),
        result: None,
        error: None,
    }
}

fn watch_notification_event(event: &PatchEvent) -> McpMessage {
    McpMessage {
        id: Uuid::new_v4(),
        stream_id: 0,
        kind: MessageKind::Notification,
        method: Some("state.patch_event".into()),
        params: Some(serde_json::json!({
            "path": event.path,
            "old_value": event.old_value,
            "new_value": event.new_value,
            "revision": event.revision,
        })),
        result: None,
        error: None,
    }
}

async fn handle_request(
    method: String,
    params: Option<serde_json::Value>,
    req_id: &Uuid,
    state: &AppState,
) -> McpMessage {
    match method.as_str() {
        "state.get" => {
            let path = params
                .as_ref()
                .and_then(|p| p.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let store = state.store.lock().await;
            match store.get_stored(path) {
                Some(sv) => success_response(
                    req_id,
                    serde_json::json!({ "value": sv.value, "revision": sv.revision }),
                ),
                None => success_response(req_id, serde_json::json!({ "value": null })),
            }
        }
        "state.set" => {
            let path = params
                .as_ref()
                .and_then(|p| p.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let value = params.as_ref().and_then(|p| p.get("value").cloned());
            let revision = state.store.lock().await.set(path, value);
            success_response(req_id, serde_json::json!({ "revision": revision }))
        }
        "state.patch" => {
            let ops = params
                .as_ref()
                .and_then(|p| p.get("ops"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let revision = state.store.lock().await.patch(&ops);
            success_response(req_id, serde_json::json!({ "revision": revision }))
        }
        "state.list" => {
            let prefix = params
                .as_ref()
                .and_then(|p| p.get("prefix"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let store = state.store.lock().await;
            let paths: Vec<serde_json::Value> = store
                .list_prefix(prefix)
                .into_iter()
                .map(|(path, sv)| {
                    serde_json::json!({ "path": path, "value": sv.value, "revision": sv.revision })
                })
                .collect();
            success_response(req_id, serde_json::json!({ "paths": paths }))
        }
        "state.get_revision" => {
            let revision = state.store.lock().await.revision();
            success_response(req_id, serde_json::json!({ "revision": revision }))
        }
        "state.stats" => {
            let revision = state.store.lock().await.revision();
            let subs = state.subscriptions.read().await.len();
            success_response(
                req_id,
                serde_json::json!({
                    "revision": revision,
                    "subscriptions": subs,
                }),
            )
        }
        _ => error_response(
            req_id,
            "E_NOT_FOUND",
            &format!("Unknown method: {}", method),
        ),
    }
}

fn success_response(id: &Uuid, result: serde_json::Value) -> McpMessage {
    McpMessage {
        id: *id,
        stream_id: 0,
        kind: MessageKind::Response,
        method: None,
        params: None,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: &Uuid, code: &str, message: &str) -> McpMessage {
    McpMessage {
        id: *id,
        stream_id: 0,
        kind: MessageKind::Response,
        method: None,
        params: None,
        result: None,
        error: Some(McpError {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> AppState {
        std::env::set_var("STATE_STORE_BACKEND", "memory");
        AppState {
            store: Arc::new(Mutex::new(Store::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[tokio::test]
    async fn state_get_missing_path_returns_null() {
        let state = test_state();
        let id = Uuid::new_v4();
        let resp = handle_request(
            "state.get".into(),
            Some(serde_json::json!({ "path": "ui.missing" })),
            &id,
            &state,
        )
        .await;
        assert_eq!(resp.id, id);
        assert!(resp.error.is_none());
        assert_eq!(
            resp.result
                .as_ref()
                .and_then(|v| v.get("value"))
                .unwrap_or(&serde_json::Value::Null),
            &serde_json::Value::Null
        );
    }

    #[tokio::test]
    async fn state_set_and_get_roundtrip() {
        let state = test_state();
        let id = Uuid::new_v4();
        let set_resp = handle_request(
            "state.set".into(),
            Some(serde_json::json!({ "path": "ui.task.name", "value": "Ada" })),
            &id,
            &state,
        )
        .await;
        assert!(set_resp.error.is_none());
        assert!(set_resp
            .result
            .as_ref()
            .and_then(|v| v.get("revision"))
            .and_then(|v| v.as_u64())
            .is_some());

        let get_id = Uuid::new_v4();
        let get_resp = handle_request(
            "state.get".into(),
            Some(serde_json::json!({ "path": "ui.task.name" })),
            &get_id,
            &state,
        )
        .await;
        assert!(get_resp.error.is_none());
        assert_eq!(
            get_resp
                .result
                .as_ref()
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_str()),
            Some("Ada")
        );
    }

    #[tokio::test]
    async fn state_patch_increment_updates_value() {
        let state = test_state();
        let id = Uuid::new_v4();
        handle_request(
            "state.set".into(),
            Some(serde_json::json!({ "path": "ui.counter", "value": 3 })),
            &id,
            &state,
        )
        .await;

        let patch_id = Uuid::new_v4();
        let patch_resp = handle_request(
            "state.patch".into(),
            Some(serde_json::json!({
                "ops": [{ "op": "INCREMENT", "path": "ui.counter", "value": 2 }]
            })),
            &patch_id,
            &state,
        )
        .await;
        assert!(patch_resp.error.is_none());

        let get_resp = handle_request(
            "state.get".into(),
            Some(serde_json::json!({ "path": "ui.counter" })),
            &Uuid::new_v4(),
            &state,
        )
        .await;
        assert_eq!(
            get_resp
                .result
                .as_ref()
                .and_then(|v| v.get("value"))
                .and_then(|v| v.as_i64()),
            Some(5)
        );
    }

    #[tokio::test]
    async fn state_list_returns_prefix_matches() {
        let state = test_state();
        let id = Uuid::new_v4();
        handle_request(
            "state.set".into(),
            Some(serde_json::json!({ "path": "ui.task.a", "value": 1 })),
            &id,
            &state,
        )
        .await;
        handle_request(
            "state.set".into(),
            Some(serde_json::json!({ "path": "ui.task.b", "value": 2 })),
            &Uuid::new_v4(),
            &state,
        )
        .await;
        handle_request(
            "state.set".into(),
            Some(serde_json::json!({ "path": "other.x", "value": 9 })),
            &Uuid::new_v4(),
            &state,
        )
        .await;

        let list_resp = handle_request(
            "state.list".into(),
            Some(serde_json::json!({ "prefix": "ui.task." })),
            &Uuid::new_v4(),
            &state,
        )
        .await;
        assert!(list_resp.error.is_none());
        let paths = list_resp
            .result
            .as_ref()
            .and_then(|v| v.get("paths"))
            .and_then(|v| v.as_array())
            .expect("paths array");
        assert_eq!(paths.len(), 2);
        let path_names: Vec<&str> = paths
            .iter()
            .filter_map(|p| p.get("path").and_then(|v| v.as_str()))
            .collect();
        assert!(path_names.contains(&"ui.task.a"));
        assert!(path_names.contains(&"ui.task.b"));
    }

    #[tokio::test]
    async fn state_stats_reports_revision_and_subscriptions() {
        let state = test_state();
        handle_request(
            "state.set".into(),
            Some(serde_json::json!({ "path": "ui.x", "value": true })),
            &Uuid::new_v4(),
            &state,
        )
        .await;

        let resp = handle_request("state.stats".into(), None, &Uuid::new_v4(), &state).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("result");
        assert!(result.get("revision").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
        assert_eq!(
            result.get("subscriptions").and_then(|v| v.as_u64()),
            Some(0)
        );
    }

    #[tokio::test]
    async fn unknown_method_returns_not_found() {
        let state = test_state();
        let id = Uuid::new_v4();
        let resp = handle_request("state.nope".into(), None, &id, &state).await;
        assert_eq!(resp.id, id);
        assert_eq!(
            resp.error.as_ref().map(|e| e.code.as_str()),
            Some("E_NOT_FOUND")
        );
    }
}
