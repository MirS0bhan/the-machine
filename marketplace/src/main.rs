//! MCP capability marketplace: signed bundles, install flow, CONFIRM gate.

use common::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
struct CapabilityBundle {
    id: String,
    name: String,
    version: String,
    description: String,
    signature: String,
    lambdas: Vec<Value>,
    ui_patches: Vec<Value>,
    policy_rules: Vec<Value>,
}

struct AppState {
    bundles: Mutex<HashMap<String, CapabilityBundle>>,
    installed: Mutex<Vec<String>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();
    info!("Marketplace daemon starting");
    let state = Arc::new(Mutex::new(AppState {
        bundles: Mutex::new(HashMap::new()),
        installed: Mutex::new(Vec::new()),
    }));
    seed_builtin_bundle(&state).await;

    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let socket_path = format!("{}/marketplace.sock", socket_dir);
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let _ = tokio::fs::remove_file(&socket_path).await;
    let listener = UnixListener::bind(&socket_path)?;
    info!("Marketplace listening on {}", socket_path);
    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(handle_connection(stream, state));
    }
}

async fn seed_builtin_bundle(state: &Arc<Mutex<AppState>>) {
    let bundle = CapabilityBundle {
        id: "calc-pack-v1".into(),
        name: "Calculator Pack".into(),
        version: "1.0.0".into(),
        description: "Basic calculator lambda + UI button".into(),
        signature: sign_bundle("calc-pack-v1"),
        lambdas: vec![json!({
            "name": "calc.eval",
            "description": "Evaluate math expressions",
            "source": "#!/usr/bin/env python3\nimport json,sys\nprint(json.dumps({'result': eval(json.loads(sys.stdin.read() or '{}').get('expression','1+1'), {'__builtins__':{}})}))",
            "language": "python",
            "exposes_mcp": ["calc.*"]
        })],
        ui_patches: vec![json!({
            "op": "insert",
            "anchor": "ui.root",
            "node": { "id": "ui.calc_btn", "type": "button", "props": { "label": "Calculate" } }
        })],
        policy_rules: vec![json!({ "capability": "CAP_IPC_CALL", "path": "calc.*", "effect": "ALLOW" })],
    };
    state.lock().await.bundles.lock().await.insert(bundle.id.clone(), bundle);
}

fn sign_bundle(id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(id.as_bytes());
    hasher.update(b"the-machine-marketplace-v1");
    hex::encode(hasher.finalize())
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
                let value: Value = match serde_json::from_str(line.trim()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
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
        None => return err(id, "E_INVALID", "missing method"),
    };
    let params = value.get("params").cloned().unwrap_or(Value::Null);
    match method.as_str() {
        "marketplace.list" => {
            let st = state.lock().await;
            let bundles = st.bundles.lock().await;
            let items: Vec<Value> = bundles
                .values()
                .map(|b| {
                    json!({
                        "id": b.id,
                        "name": b.name,
                        "version": b.version,
                        "description": b.description,
                    })
                })
                .collect();
            ok(id, json!({ "bundles": items }))
        }
        "marketplace.install" => {
            let bundle_id = params.get("bundle_id").and_then(|v| v.as_str()).unwrap_or("");
            let confirm = params.get("confirm").and_then(|v| v.as_bool()).unwrap_or(false);
            let st = state.lock().await;
            let bundle = st.bundles.lock().await.get(bundle_id).cloned();
            let Some(bundle) = bundle else {
                return err(id, "E_NOT_FOUND", "bundle not found");
            };
            if !confirm {
                return ok(id, json!({ "status": "CONFIRM_REQUIRED", "bundle": bundle.name }));
            }
            for lambda in &bundle.lambdas {
                let _ = bus_call(
                    "lambda.register",
                    json!({ "manifest": lambda }),
                )
                .await;
            }
            for patch in &bundle.ui_patches {
                let _ = bus_call("ui.patch", json!({ "ops": [patch] })).await;
            }
            st.installed.lock().await.push(bundle_id.to_string());
            ok(id, json!({ "installed": bundle_id, "status": "ok" }))
        }
        "marketplace.installed" => {
            let st = state.lock().await;
            let list = st.installed.lock().await.clone();
            ok(id, json!({ "installed": list }))
        }
        _ => err(id, "E_NOT_FOUND", &format!("unknown method: {}", method)),
    }
}

async fn bus_call(method: &str, params: Value) -> Option<Value> {
    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let path = format!("{}/mcp-bus.sock", socket_dir);
    let mut stream = tokio::net::UnixStream::connect(&path).await.ok()?;
    let req = json!({
        "id": Uuid::new_v4(),
        "kind": "Request",
        "method": method,
        "params": params,
    });
    let mut bytes = serde_json::to_vec(&req).ok()?;
    bytes.push(b'\n');
    stream.write_all(&bytes).await.ok()?;
    let mut buf = vec![0u8; 65536];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await.ok()?;
    let resp: Value = serde_json::from_slice(&buf[..n]).ok()?;
    resp.get("result").cloned()
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "id": id, "result": result })
}
fn err(id: Value, code: &str, message: &str) -> Value {
    json!({ "id": id, "error": { "code": code, "message": message } })
}
