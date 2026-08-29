//! System Daemon - Raw I/O Ownership & Narrow Kernel-Parameter MCP Surface

use common::*;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

mod input;
mod kernel;

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
                handle_request(method, msg.params, state).await
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
    state: &AppState,
) -> McpMessage {
    let id = Uuid::new_v4();

    match method.as_str() {
        // Read-only queries
        "power.get_profile" => {
            let profile = state.kernel_handler.lock().await.get_power_profile();
            success_response(&id, serde_json::json!({ "profile": profile }))
        }
        "display.get_modes" => {
            let modes = state.kernel_handler.lock().await.get_display_modes();
            success_response(&id, serde_json::json!({ "modes": modes }))
        }
        "net.list_interfaces" => {
            let interfaces = state.kernel_handler.lock().await.list_interfaces();
            success_response(&id, serde_json::json!({ "interfaces": interfaces }))
        }
        "audio.list_devices" => {
            let devices = state.kernel_handler.lock().await.list_audio_devices();
            success_response(&id, serde_json::json!({ "devices": devices }))
        }
        "system-daemon.stats" => {
            let stats = state.stats.read().await;
            success_response(
                &id,
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
        "power.set_profile" => {
            if let Some(params) = params {
                if let Some(profile) = params.get("profile").and_then(|v| v.as_str()) {
                    match state
                        .kernel_handler
                        .lock()
                        .await
                        .set_power_profile(profile)
                        .await
                    {
                        Ok(_) => success_response(&id, serde_json::json!({})),
                        Err(e) => error_response(&id, "E_POLICY_DENIED", &e),
                    }
                } else {
                    error_response(&id, "E_INVALID_REQUEST", "Missing profile parameter")
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "display.set_mode" => {
            if let Some(params) = params {
                let width = params.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let height = params.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let refresh = params
                    .get("refresh")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as f32;
                match state
                    .kernel_handler
                    .lock()
                    .await
                    .set_display_mode(width, height, refresh)
                    .await
                {
                    Ok(_) => success_response(&id, serde_json::json!({})),
                    Err(e) => error_response(&id, "E_POLICY_DENIED", &e),
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "net.set_interface_state" => {
            if let Some(params) = params {
                if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                    if let Some(state_str) = params.get("state").and_then(|v| v.as_str()) {
                        match state
                            .kernel_handler
                            .lock()
                            .await
                            .set_interface_state(name, state_str)
                            .await
                        {
                            Ok(_) => success_response(&id, serde_json::json!({})),
                            Err(e) => error_response(&id, "E_POLICY_DENIED", &e),
                        }
                    } else {
                        error_response(&id, "E_INVALID_REQUEST", "Missing state parameter")
                    }
                } else {
                    error_response(&id, "E_INVALID_REQUEST", "Missing name parameter")
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "net.connect_wifi" => {
            if let Some(params) = params {
                if let Some(ssid) = params.get("ssid").and_then(|v| v.as_str()) {
                    let credential_ref = params
                        .get("credential_ref")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    match state
                        .kernel_handler
                        .lock()
                        .await
                        .connect_wifi(ssid, credential_ref)
                        .await
                    {
                        Ok(status) => {
                            success_response(&id, serde_json::json!({ "status": status }))
                        }
                        Err(e) => error_response(&id, "E_POLICY_DENIED", &e),
                    }
                } else {
                    error_response(&id, "E_INVALID_REQUEST", "Missing ssid parameter")
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "audio.set_default" => {
            if let Some(params) = params {
                if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                    match state
                        .kernel_handler
                        .lock()
                        .await
                        .set_default_audio(name)
                        .await
                    {
                        Ok(_) => success_response(&id, serde_json::json!({})),
                        Err(e) => error_response(&id, "E_POLICY_DENIED", &e),
                    }
                } else {
                    error_response(&id, "E_INVALID_REQUEST", "Missing name parameter")
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
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
