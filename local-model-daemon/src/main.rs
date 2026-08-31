mod engine;
mod gguf;

use engine::Engine;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{error, info};

struct AppState {
    engine: Engine,
    model_path: String,
    native_loaded: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();
    info!("Local Model daemon starting");
    let model_path =
        std::env::var("LOCAL_MODEL_PATH").unwrap_or_else(|_| "/models/machine-tiny.gguf".into());
    let native = gguf::NativeModel::open(&model_path);
    if native.is_some() {
        info!("GGUF model loaded from {}", model_path);
    }
    let state = Arc::new(Mutex::new(AppState {
        engine: Engine::new(),
        model_path,
        native_loaded: native.is_some(),
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

async fn handle_connection(stream: tokio::net::UnixStream, state: Arc<Mutex<AppState>>) {
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
        "localmodel.health" => ok(
            id,
            json!({
                "status": if st.native_loaded || !st.engine.health().get("status").and_then(|v| v.as_str()).unwrap_or("").contains("stub") { "ready" } else { "stub" },
                "model_path": st.model_path,
                "gguf_loaded": st.native_loaded,
                "backend": st.engine.health(),
            }),
        ),
        "localmodel.complete" => {
            let prompt = params.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
            let max_tokens = params
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(512) as u32;
            let temperature = params
                .get("temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.7) as f32;
            let text = st.engine.complete(prompt, max_tokens, temperature).await;
            ok(id, json!({ "text": text, "privacy_tag": "none" }))
        }
        "localmodel.classify_intent" => {
            let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let category = params
                .get("category")
                .and_then(|v| v.as_str())
                .unwrap_or("input");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state(native_loaded: bool) -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState {
            engine: Engine::new(),
            model_path: "/models/test.gguf".into(),
            native_loaded,
        }))
    }

    fn mcp_request(id: u64, method: &str, params: Value) -> Value {
        json!({ "id": id, "method": method, "params": params })
    }

    #[tokio::test]
    async fn localmodel_health_reports_stub_backend_without_gguf() {
        let state = test_state(false);
        let resp = handle_request(mcp_request(1, "localmodel.health", json!({})), state).await;
        assert!(resp.get("error").is_none());
        let result = resp.get("result").expect("result");
        assert_eq!(
            result.get("model_path").and_then(|v| v.as_str()),
            Some("/models/test.gguf")
        );
        assert_eq!(
            result.get("gguf_loaded").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(result.get("status").and_then(|v| v.as_str()), Some("stub"));
        assert!(result.get("backend").is_some());
    }

    #[tokio::test]
    async fn localmodel_health_reports_ready_when_gguf_loaded() {
        let state = test_state(true);
        let resp = handle_request(mcp_request(2, "localmodel.health", json!({})), state).await;
        assert!(resp.get("error").is_none());
        let result = resp.get("result").expect("result");
        assert_eq!(
            result.get("gguf_loaded").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(result.get("status").and_then(|v| v.as_str()), Some("ready"));
    }

    #[tokio::test]
    async fn localmodel_complete_returns_text_and_privacy_tag() {
        let state = test_state(false);
        let resp = handle_request(
            mcp_request(
                3,
                "localmodel.complete",
                json!({ "prompt": "hello", "max_tokens": 32, "temperature": 0.5 }),
            ),
            state,
        )
        .await;
        assert!(resp.get("error").is_none());
        let result = resp.get("result").expect("result");
        assert!(result
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("hello"));
        assert_eq!(
            result.get("privacy_tag").and_then(|v| v.as_str()),
            Some("none")
        );
    }

    #[tokio::test]
    async fn localmodel_classify_intent_returns_routing_fields() {
        let state = test_state(false);
        let resp = handle_request(
            mcp_request(
                4,
                "localmodel.classify_intent",
                json!({ "text": "open settings", "category": "input" }),
            ),
            state,
        )
        .await;
        assert!(resp.get("error").is_none());
        let result = resp.get("result").expect("result");
        assert!(result.get("intent").and_then(|v| v.as_str()).is_some());
        assert!(result.get("confidence").and_then(|v| v.as_f64()).is_some());
        assert!(result.get("complexity").and_then(|v| v.as_str()).is_some());
        assert!(result.get("routing").and_then(|v| v.as_str()).is_some());
        assert!(result
            .get("requires_cloud")
            .and_then(|v| v.as_bool())
            .is_some());
        assert_eq!(
            result.get("privacy_tag").and_then(|v| v.as_str()),
            Some("none")
        );
    }

    #[tokio::test]
    async fn localmodel_embed_returns_embedding_vector() {
        let state = test_state(false);
        let resp = handle_request(
            mcp_request(5, "localmodel.embed", json!({ "text": "embed me" })),
            state,
        )
        .await;
        assert!(resp.get("error").is_none());
        let result = resp.get("result").expect("result");
        let embedding = result
            .get("embedding")
            .and_then(|v| v.as_array())
            .expect("embedding array");
        assert!(!embedding.is_empty());
        assert_eq!(
            result.get("privacy_tag").and_then(|v| v.as_str()),
            Some("none")
        );
    }

    #[tokio::test]
    async fn missing_method_returns_invalid_request() {
        let state = test_state(false);
        let resp = handle_request(json!({ "id": 6 }), state).await;
        assert_eq!(
            resp.get("error")
                .and_then(|e| e.get("code"))
                .and_then(|v| v.as_str()),
            Some("E_INVALID_REQUEST")
        );
    }

    #[tokio::test]
    async fn unknown_method_returns_not_found() {
        let state = test_state(false);
        let resp = handle_request(mcp_request(7, "localmodel.nope", json!({})), state).await;
        assert_eq!(
            resp.get("error")
                .and_then(|e| e.get("code"))
                .and_then(|v| v.as_str()),
            Some("E_NOT_FOUND")
        );
    }
}
