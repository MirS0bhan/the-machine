mod engine;

use common::*;
use engine::Engine;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{error, info};
use uuid::Uuid;

struct AppState {
    engine: Engine,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();
    info!("Local Model daemon starting");
    let state = Arc::new(Mutex::new(AppState {
        engine: Engine::new(),
    }));
    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let socket_path = format!("{}/local-model-daemon.sock", socket_dir);
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let _ = tokio::fs::remove_file(&socket_path).await;
    let listener = UnixListener::bind(&socket_path)?;
    info!("Listening on {}", socket_path);
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state = state.clone();
                tokio::spawn(handle_connection(stream, state));
            }
            Err(e) => error!("accept: {}", e),
        }
    }
}

async fn handle_connection(mut stream: tokio::net::UnixStream, state: Arc<Mutex<AppState>>) {
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
                let value: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let is_request = value
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k != "Notification")
                    .unwrap_or(true);
                if !is_request {
                    continue;
                }
                let response = handle_request(value, state.clone()).await;
                if let Ok(bytes) = serde_json::to_vec(&response) {
                    let mut buf = bytes;
                    buf.push(b'\n');
                    if writer.write_all(&buf).await.is_err() {
                        break;
                    }
                }
            }
            Err(e) => {
                error!("read: {}", e);
                break;
            }
        }
    }
}

async fn handle_request(value: Value, state: Arc<Mutex<AppState>>) -> Value {
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    let method = match value.get("method").and_then(|m| m.as_str()) {
        Some(m) => m.to_string(),
        None => return err(id, "E_INVALID_REQUEST", "missing method"),
    };
    let params = value.get("params").cloned().unwrap_or(Value::Null);
    let st = state.lock().await;
    match method.as_str() {
        "localmodel.health" => ok(id, st.engine.health()),
        "localmodel.complete" => {
            let prompt = params.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
            let max_tokens = params.get("max_tokens").and_then(|v| v.as_u64()).unwrap_or(512) as u32;
            let temperature = params.get("temperature").and_then(|v| v.as_f64()).unwrap_or(0.7) as f32;
            let text = st.engine.complete(prompt, max_tokens, temperature).await;
            ok(id, json!({ "text": text, "privacy_tag": "none" }))
        }
        "localmodel.classify_intent" => {
            let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let category = params.get("category").and_then(|v| v.as_str()).unwrap_or("input");
            let (intent, confidence, complexity, routing, requires_cloud) =
                st.engine.classify_intent(text, category).await;
            ok(
                id,
                json!({
                    "intent": intent,
                    "confidence": confidence,
                    "complexity": complexity,
                    "routing": routing,
                    "requires_cloud": requires_cloud,
                    "privacy_tag": "none",
                }),
            )
        }
        "localmodel.embed" => {
            let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let embedding = st.engine.embed(text).await;
            ok(id, json!({ "embedding": embedding, "privacy_tag": "none" }))
        }
        _ => err(id, "E_NOT_FOUND", &format!("unknown method: {}", method)),
    }
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "id": id, "result": result })
}
fn err(id: Value, code: &str, message: &str) -> Value {
    json!({ "id": id, "error": { "code": code, "message": message } })
}
