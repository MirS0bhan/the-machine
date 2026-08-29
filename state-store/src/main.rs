//! State Store - UI State Tree, System/Task State, Persistence & Subscriptions

use common::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, RwLock, broadcast};
use tracing::{info, warn, error};

mod storage;
use storage::Store;

#[derive(Clone)]
struct AppState {
    store: Arc<Mutex<Store>>,
    subscriptions: Arc<RwLock<HashMap<String, Vec<broadcast::Sender<PatchEvent>>>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PatchEvent {
    path: String,
    old_value: Option<serde_json::Value>,
    new_value: Option<serde_json::Value>,
    revision: u64,
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

    let socket_path = "/run/the-machine/state-store.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;
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
                if let Ok(response) = process_message(&line, &state).await {
                    if let Err(e) = writer.write_all(response.as_bytes()).await {
                        error!("Write error: {}", e);
                        break;
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

async fn process_message(line: &str, state: &AppState) -> anyhow::Result<String> {
    let msg: McpMessage = serde_json::from_str(line.trim())?;
    
    let response = match msg.kind {
        MessageKind::Request => {
            if let Some(method) = msg.method {
                handle_request(method, msg.params, state).await
            } else {
                error_response(&msg.id, "E_INVALID_REQUEST", "Missing method")
            }
        }
        _ => error_response(&msg.id, "E_INVALID_REQUEST", "Only requests supported"),
    };

    Ok(serde_json::to_string(&response)? + "\n")
}

async fn handle_request(method: String, params: Option<serde_json::Value>, state: &AppState) -> McpMessage {
    let id = Uuid::new_v4();
    
    match method.as_str() {
        "state.get" => {
            if let Some(params) = params {
                if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
                    let value = state.store.lock().await.get(path);
                    success_response(&id, serde_json::json!({ "value": value }))
                } else {
                    error_response(&id, "E_INVALID_REQUEST", "Missing path parameter")
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "state.set" => {
            if let Some(params) = params {
                let path = params.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let value = params.get("value").cloned();
                let revision = state.store.lock().await.set(path, value);
                success_response(&id, serde_json::json!({ "revision": revision }))
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "state.patch" => {
            if let Some(params) = params {
                if let Some(ops) = params.get("ops").and_then(|v| v.as_array()) {
                    let revision = state.store.lock().await.patch(ops);
                    success_response(&id, serde_json::json!({ "revision": revision }))
                } else {
                    error_response(&id, "E_INVALID_REQUEST", "Missing ops parameter")
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "state.watch" => {
            if let Some(params) = params {
                if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
                    let (tx, mut rx) = broadcast::channel(1024);
                    
                    // Register subscription
                    state.subscriptions.write().await
                        .entry(path.to_string())
                        .or_default()
                        .push(tx);
                    
                    // For now, just return the subscription info
                    success_response(&id, serde_json::json!({ "subscription_id": Uuid::new_v4() }))
                } else {
                    error_response(&id, "E_INVALID_REQUEST", "Missing path parameter")
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "state.get_revision" => {
            let revision = state.store.lock().await.revision();
            success_response(&id, serde_json::json!({ "revision": revision }))
        }
        _ => error_response(&id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
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