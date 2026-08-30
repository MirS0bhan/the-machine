//! System Daemon - Raw I/O Ownership & Narrow Kernel-Parameter MCP Surface

use common::*;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

mod audio;
mod display;
mod input;
mod kernel;
mod net;
mod netlink;
mod power;
mod wifi;

#[derive(Clone)]
struct AppState {
    kernel_handler: Arc<Mutex<kernel::KernelHandler>>,
    input_forwarder: Arc<Mutex<input::InputForwarder>>,
    stats: Arc<RwLock<SystemStats>>,
}

#[derive(Default)]
struct SystemStats {
    input_events_forwarded: u64,
    input_events_dropped: u64,
    kernel_ops_executed: u64,
    kernel_ops_denied: u64,
    broker_status: String,
    uptime: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    info!("Starting System Daemon");

    let state = AppState {
        kernel_handler: Arc::new(Mutex::new(kernel::KernelHandler::new())),
        input_forwarder: Arc::new(Mutex::new(input::InputForwarder::new())),
        stats: Arc::new(RwLock::new(SystemStats::default())),
    };

    // Start input forwarding task
    let input_state = state.clone();
    tokio::spawn(async move {
        if let Err(e) = input_state.input_forwarder.lock().await.run().await {
            error!("Input forwarder error: {}", e);
        }
    });

    // Start stats updater
    let stats_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        let start = std::time::Instant::now();
        loop {
            interval.tick().await;
            let mut stats = stats_state.stats.write().await;
            stats.uptime = start.elapsed().as_secs_f64();
        }
    });

    // Start MCP server
    let socket_path = common::component_socket("system-daemon");
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    info!("System Daemon listening on {}", socket_path);

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
            Ok(0) => break, // EOF
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
                handle_request(method, msg.params, &msg.id, state).await
            } else {
                error_response(&msg.id, "E_INVALID_REQUEST", "Missing method")
            }
        }
        _ => error_response(&msg.id, "E_INVALID_REQUEST", "Only requests supported"),
    };

    Ok(serde_json::to_string(&response)? + "\n")
}

async fn handle_request(
    method: String,
    params: Option<serde_json::Value>,
    id: &Uuid,
    state: &AppState,
) -> McpMessage {
    match method.as_str() {
        // Read-only queries
        "power.get_profile" => {
            let profile = state.kernel_handler.lock().await.get_power_profile();
            success_response(id, serde_json::json!({ "profile": profile }))
        }
        "display.get_modes" => {
            let modes = state.kernel_handler.lock().await.get_display_modes();
            success_response(id, serde_json::json!({ "modes": modes }))
        }
        "net.list_interfaces" => {
            let interfaces = state.kernel_handler.lock().await.list_interfaces().await;
            success_response(id, serde_json::json!({ "interfaces": interfaces }))
        }
        "audio.list_devices" => {
            let devices = state.kernel_handler.lock().await.list_audio_devices();
            success_response(id, serde_json::json!({ "devices": devices }))
        }
        "net.get_wifi_status" => {
            let status = state.kernel_handler.lock().await.get_wifi_status();
            success_response(id, status)
        }
        "system-daemon.stats" => {
            let stats = state.stats.read().await;
            success_response(
                id,
                serde_json::json!({
                    "input_events_forwarded": stats.input_events_forwarded,
                    "input_events_dropped": stats.input_events_dropped,
                    "kernel_ops_executed": stats.kernel_ops_executed,
                    "kernel_ops_denied": stats.kernel_ops_denied,
                    "broker_status": stats.broker_status,
                    "uptime": stats.uptime
                }),
            )
        }
        // Mutations (require grant token)
        "power.set_profile"
        | "display.set_mode"
        | "net.set_interface_state"
        | "net.connect_wifi"
        | "audio.set_default" => {
            if let Err(e) = require_grant(params.as_ref(), method.as_str()) {
                state.stats.write().await.kernel_ops_denied += 1;
                return error_response(id, "E_POLICY_DENIED", &e);
            }
            handle_mutation(method.as_str(), params, id, state).await
        }
        _ => error_response(id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
    }
}

async fn handle_mutation(
    method: &str,
    params: Option<serde_json::Value>,
    id: &Uuid,
    state: &AppState,
) -> McpMessage {
    let Some(params) = params else {
        return error_response(id, "E_INVALID_REQUEST", "Missing parameters");
    };
    let outcome = match method {
        "power.set_profile" => {
            let Some(profile) = params.get("profile").and_then(|v| v.as_str()) else {
                return error_response(id, "E_INVALID_REQUEST", "Missing profile parameter");
            };
            state
                .kernel_handler
                .lock()
                .await
                .set_power_profile(profile)
                .await
                .map(|_| serde_json::json!({}))
        }
        "display.set_mode" => {
            let width = params.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let height = params.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
            let refresh = params
                .get("refresh")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            state
                .kernel_handler
                .lock()
                .await
                .set_display_mode(width, height, refresh)
                .await
                .map(|_| serde_json::json!({}))
        }
        "net.set_interface_state" => {
            let Some(name) = params.get("name").and_then(|v| v.as_str()) else {
                return error_response(id, "E_INVALID_REQUEST", "Missing name parameter");
            };
            let Some(state_str) = params.get("state").and_then(|v| v.as_str()) else {
                return error_response(id, "E_INVALID_REQUEST", "Missing state parameter");
            };
            state
                .kernel_handler
                .lock()
                .await
                .set_interface_state(name, state_str)
                .await
                .map(|_| serde_json::json!({}))
        }
        "net.connect_wifi" => {
            let Some(ssid) = params.get("ssid").and_then(|v| v.as_str()) else {
                return error_response(id, "E_INVALID_REQUEST", "Missing ssid parameter");
            };
            let credential_ref = params
                .get("credential_ref")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            state
                .kernel_handler
                .lock()
                .await
                .connect_wifi(ssid, credential_ref)
                .await
                .map(|status| serde_json::json!({ "status": status }))
        }
        "audio.set_default" => {
            let Some(name) = params.get("name").and_then(|v| v.as_str()) else {
                return error_response(id, "E_INVALID_REQUEST", "Missing name parameter");
            };
            state
                .kernel_handler
                .lock()
                .await
                .set_default_audio(name)
                .await
                .map(|_| serde_json::json!({}))
        }
        _ => return error_response(id, "E_NOT_FOUND", &format!("Unknown method: {method}")),
    };
    match outcome {
        Ok(result) => {
            state.stats.write().await.kernel_ops_executed += 1;
            success_response(id, result)
        }
        Err(e) => {
            state.stats.write().await.kernel_ops_denied += 1;
            error_response(id, "E_UNAVAILABLE", &e)
        }
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
        AppState {
            kernel_handler: Arc::new(Mutex::new(kernel::KernelHandler::new())),
            input_forwarder: Arc::new(Mutex::new(input::InputForwarder::new())),
            stats: Arc::new(RwLock::new(SystemStats::default())),
        }
    }

    #[tokio::test]
    async fn mutation_without_token_is_denied_and_keeps_request_id() {
        let state = test_state();
        let id = Uuid::new_v4();
        let resp = handle_request(
            "power.set_profile".into(),
            Some(serde_json::json!({ "profile": "powersave" })),
            &id,
            &state,
        )
        .await;
        assert_eq!(resp.id, id);
        let err = resp.error.expect("expected denial");
        assert_eq!(err.code, "E_POLICY_DENIED");
        assert_eq!(state.stats.read().await.kernel_ops_denied, 1);
    }

    #[tokio::test]
    async fn mutation_with_valid_token_runs() {
        let state = test_state();
        let id = Uuid::new_v4();
        let token = shared_verifier().issue_token(
            GrantScope {
                method: "power.set_profile".into(),
                request_hash: "t".into(),
                requester_identity: "test".into(),
            },
            60,
        );
        let resp = handle_request(
            "power.set_profile".into(),
            Some(serde_json::json!({ "profile": "powersave", "token": token })),
            &id,
            &state,
        )
        .await;
        assert_eq!(resp.id, id);
        let err_code = resp.error.as_ref().map(|e| e.code.as_str());
        assert!(
            resp.error.is_none() || err_code == Some("E_UNAVAILABLE"),
            "token should pass; got {:?}",
            resp.error
        );
        if resp.error.is_none() {
            assert_eq!(state.stats.read().await.kernel_ops_executed, 1);
        }
    }

    #[tokio::test]
    async fn wifi_status_is_implemented() {
        let state = test_state();
        let id = Uuid::new_v4();
        let resp = handle_request("net.get_wifi_status".into(), None, &id, &state).await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        let status = result.get("status").and_then(|v| v.as_str());
        assert!(status == Some("disconnected") || status == Some("associated"));
    }

    #[tokio::test]
    async fn display_set_mode_without_drm_returns_unavailable() {
        let state = test_state();
        let id = Uuid::new_v4();
        std::env::set_var("THE_MACHINE_DRM_DEVICE", "/tmp/the-machine-no-drm-card0");
        let token = shared_verifier().issue_token(
            GrantScope {
                method: "display.set_mode".into(),
                request_hash: "t".into(),
                requester_identity: "test".into(),
            },
            60,
        );
        let resp = handle_request(
            "display.set_mode".into(),
            Some(serde_json::json!({
                "width": 1920,
                "height": 1080,
                "refresh": 60.0,
                "token": token,
            })),
            &id,
            &state,
        )
        .await;
        std::env::remove_var("THE_MACHINE_DRM_DEVICE");
        let err = resp.error.expect("expected unavailable without DRM");
        assert_eq!(err.code, "E_UNAVAILABLE");
    }
}
