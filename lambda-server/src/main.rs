//! Lambda Server — local serverless runtime with per-invocation sandboxing.
//!
//! **Migration note:** Python `lambda-server/*.py` is canonical for tests and agent dev.
//! This Rust crate is canonical for ISO boot (real seccomp/namespaces). They do not share
//! a function registry. See `docs/guides/python-rust-overlap.md`.
//!
//! Every function runs sandboxed (see `sandbox.rs`): its own namespaces, all
//! capabilities dropped, and a seccomp *allowlist* derived from its declared
//! capabilities. Functions that declare the same `ipc_group` share one IPC
//! namespace so they can call each other; everything else is isolated.
//!
//! MCP methods: lambda.register, lambda.invoke, lambda.lease,
//! lambda.renew_lease, lambda.status, lambda.list, lambda.search,
//! lambda.health, lambda.stop.

mod sandbox;
use sandbox::*;

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    base_image: String,
    entrypoint: String,
    #[serde(default)]
    persistent: bool,
    #[serde(default)]
    ipc_group: Option<String>,
    #[serde(default)]
    timeout_ms: u64,
    #[serde(default)]
    exposes_mcp: Vec<Value>,
    #[serde(default)]
    handles_event: Vec<Value>,
}

#[derive(Debug, Clone)]
struct FunctionRecord {
    name: String,
    version: u64,
    manifest: Manifest,
    status: String,
    last_invoked: Option<i64>,
}

struct AppState {
    functions: Mutex<HashMap<String, FunctionRecord>>,
    /// MCP pattern → lambda function name (e.g. "calc.*" → "calc.eval").
    mcp_exposures: Mutex<HashMap<String, String>>,
    /// Per-group IPC namespace fd (shared across functions in a group).
    ipc_groups: Mutex<HashMap<String, i32>>,
    /// Active leases: lease_id -> persistent sandboxed process.
    leases: Mutex<HashMap<String, Persistent>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            functions: Mutex::new(HashMap::new()),
            mcp_exposures: Mutex::new(HashMap::new()),
            ipc_groups: Mutex::new(HashMap::new()),
            leases: Mutex::new(HashMap::new()),
        }
    }
}

/// Create (or reuse) a persistent IPC-namespace anchor for a group so that
/// functions in the same group join the same IPC namespace.
fn ensure_ipc_group(group: &str, anchors: &mut HashMap<String, i32>) -> i32 {
    if let Some(fd) = anchors.get(group) {
        return *fd;
    }
    unsafe {
        let pid = libc::fork();
        if pid == 0 {
            libc::unshare(libc::CLONE_NEWIPC);
            loop {
                libc::pause();
            }
        }
        let path = format!("/proc/{}/ns/ipc", pid);
        let cpath = std::ffi::CString::new(path).unwrap();
        let fd = libc::open(cpath.as_ptr(), libc::O_RDONLY, 0);
        anchors.insert(group.to_string(), fd);
        fd
    }
}

fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "id": id, "result": result })
}
fn err(id: Value, code: &str, message: &str) -> Value {
    json!({ "id": id, "error": { "code": code, "message": message } })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();
    info!("Lambda Server starting");

    let state = Arc::new(AppState::new());
    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let socket_path = format!("{}/lambda-server.sock", socket_dir);
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let _ = tokio::fs::remove_file(&socket_path).await;
    let listener = UnixListener::bind(&socket_path)?;
    info!("Lambda Server listening on {}", socket_path);

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

async fn handle_connection(mut stream: tokio::net::UnixStream, state: Arc<AppState>) {
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
                    Err(e) => {
                        warn!("invalid JSON: {}", e);
                        continue;
                    }
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

async fn handle_request(value: Value, state: Arc<AppState>) -> Value {
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    let method = match value.get("method").and_then(|m| m.as_str()) {
        Some(m) => m.to_string(),
        None => return err(id, "E_INVALID_REQUEST", "missing method"),
    };
    let params = value.get("params").cloned();

    // Bus-proxied mcp-intent call (calc.add → lambda.invoke under the hood).
    if let Some(p) = &params {
        if let (Some(lambda_name), Some(route_method)) = (
            p.get("_route_lambda").and_then(|v| v.as_str()),
            p.get("_route_method").and_then(|v| v.as_str()),
        ) {
            let mut invoke_params = p.clone();
            if let Some(obj) = invoke_params.as_object_mut() {
                obj.remove("_route_lambda");
                obj.remove("_route_method");
                obj.insert("name".into(), json!(lambda_name));
                if !obj.contains_key("payload") {
                    obj.insert("payload".into(), json!({ "method": route_method }));
                }
            }
            return invoke(Some(invoke_params), state, id).await;
        }
    }

    match method.as_str() {
        "hello" => ok(id, json!({"status": "ok"})),
        "lambda.register" => register(params, state, id).await,
        "lambda.invoke" => invoke(params, state, id).await,
        "lambda.lease" => lease(params, state, id).await,
        "lambda.renew_lease" => ok(id, json!({})),
        "lambda.status" => status(params, state, id).await,
        "lambda.list" => list(state, id).await,
        "lambda.search" => search(params, state, id).await,
        "lambda.health" => health(state, id).await,
        "lambda.stop" => stop(params, state, id).await,
        _ => err(id, "E_NOT_FOUND", &format!("unknown method: {}", method)),
    }
}

async fn register(params: Option<Value>, state: Arc<AppState>, id: Value) -> Value {
    let manifest: Manifest = match params
        .as_ref()
        .and_then(|p| p.get("manifest"))
        .and_then(|m| serde_json::from_value::<Manifest>(m.clone()).ok())
    {
        Some(m) => m,
        None => return err(id, "E_INVALID_MANIFEST", "manifest required"),
    };
    if manifest.entrypoint.is_empty() {
        return err(id, "E_INVALID_MANIFEST", "entrypoint required");
    }

    let group = manifest.ipc_group.clone();
    if let Some(g) = &group {
        let mut anchors = state.ipc_groups.lock().await;
        ensure_ipc_group(g, &mut anchors);
    }

    let rec = FunctionRecord {
        name: manifest.name.clone(),
        version: 1,
        manifest: manifest.clone(),
        status: "Ready".into(),
        last_invoked: None,
    };
    state
        .functions
        .lock()
        .await
        .insert(rec.name.clone(), rec);

    info!("registered function '{}'", manifest.name);

    // Register MCP intent routes with the bus (side effect per mcp-bus-spec §3).
    for exposure in parse_string_list(&manifest.exposes_mcp) {
        register_bus_route("mcp-intent", &manifest.name, &exposure).await;
        state
            .mcp_exposures
            .lock()
            .await
            .insert(exposure, manifest.name.clone());
    }

    for event_key in parse_string_list(&manifest.handles_event) {
        register_bus_route("event-handler", &manifest.name, &event_key).await;
        register_event_handler(&manifest.name, &event_key).await;
    }

    ok(
        id,
        json!({ "name": manifest.name, "version": 1, "status": "Ready" }),
    )
}

async fn invoke(params: Option<Value>, state: Arc<AppState>, id: Value) -> Value {
    let p = params.unwrap_or(Value::Null);
    let name = match p.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return err(id, "E_NOT_FOUND", "name required"),
    };
    let payload = p.get("payload").cloned().unwrap_or(Value::Null);
    let lease_id = p.get("lease_id").and_then(|v| v.as_str()).map(|s| s.to_string());

    // Leased (warm) invocation.
    if let Some(lid) = lease_id {
        let mut leases = state.leases.lock().await;
        if let Some(persistent) = leases.get(&lid) {
            let data = serde_json::to_vec(&payload).unwrap_or_default();
            let res = persistent_request(persistent, &data, 10_000);
            return match res {
                Ok(out) => ok(id, json!({ "result": out.trim(), "lease_id": lid })),
                Err(e) => err(id, "E_INVOKE_FAILED", &e),
            };
        } else {
            return err(id, "E_NOT_FOUND", "unknown lease_id");
        }
    }

    let rec = {
        let fns = state.functions.lock().await;
        match fns.get(&name) {
            Some(r) => r.clone(),
            None => return err(id, "E_NOT_FOUND", "function not found"),
        }
    };

    let ipc_fd = match &rec.manifest.ipc_group {
        Some(g) => *state.ipc_groups.lock().await.get(g).unwrap_or(&-1),
        None => -1,
    };
    let caps = parse_caps(&rec.manifest.capabilities);
    let input = SandboxInput {
        entry: rec.manifest.entrypoint.clone(),
        args: vec![],
        input: serde_json::to_vec(&payload).unwrap_or_default(),
        timeout_ms: if rec.manifest.timeout_ms > 0 {
            rec.manifest.timeout_ms
        } else {
            10_000
        },
        ipc_ns_fd: ipc_fd,
        caps,
    };

    let out = match tokio::task::spawn_blocking(move || run_sandboxed(&input)).await {
        Ok(o) => o,
        Err(e) => return err(id, "E_INVOKE_FAILED", &format!("{}", e)),
    };

    {
        let mut fns = state.functions.lock().await;
        if let Some(r) = fns.get_mut(&name) {
            r.last_invoked = Some(now_ms());
        }
    }

    if let Some(e) = &out.error {
        return err(id, "E_INVOKE_FAILED", e);
    }
    let raw = out.stdout.trim();
    let result: Value = serde_json::from_str(raw)
        .unwrap_or_else(|_| json!({ "output": raw }));
    let mut resp = json!({ "result": result });
    if out.killed {
        resp["killed_by_seccomp"] = json!(true);
    }
    ok(id, resp)
}

async fn lease(params: Option<Value>, state: Arc<AppState>, id: Value) -> Value {
    let p = params.unwrap_or(Value::Null);
    let name = match p.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return err(id, "E_NOT_FOUND", "name required"),
    };
    let rec = {
        let fns = state.functions.lock().await;
        match fns.get(&name) {
            Some(r) => r.clone(),
            None => return err(id, "E_NOT_FOUND", "function not found"),
        }
    };
    let ipc_fd = match &rec.manifest.ipc_group {
        Some(g) => *state.ipc_groups.lock().await.get(g).unwrap_or(&-1),
        None => -1,
    };
    let caps = parse_caps(&rec.manifest.capabilities);
    let input = SandboxInput {
        entry: rec.manifest.entrypoint.clone(),
        args: vec![],
        input: vec![],
        timeout_ms: if rec.manifest.timeout_ms > 0 {
            rec.manifest.timeout_ms
        } else {
            10_000
        },
        ipc_ns_fd: ipc_fd,
        caps,
    };
    let persistent = match tokio::task::spawn_blocking(move || run_persistent(&input)).await {
        Ok(Some(p)) => p,
        _ => return err(id, "E_INVOKE_FAILED", "failed to start lease"),
    };
    let lid = Uuid::new_v4().to_string();
    state
        .leases
        .lock()
        .await
        .insert(lid.clone(), persistent);
    ok(
        id,
        json!({ "lease_id": lid, "socket_path": format!("/run/the-machine/leases/{}", lid) }),
    )
}

async fn status(params: Option<Value>, state: Arc<AppState>, id: Value) -> Value {
    let name = match params
        .as_ref()
        .and_then(|p| p.get("name"))
        .and_then(|v| v.as_str())
    {
        Some(n) => n.to_string(),
        None => return err(id, "E_NOT_FOUND", "name required"),
    };
    let fns = state.functions.lock().await;
    match fns.get(&name) {
        Some(r) => ok(
            id,
            json!({
                "name": r.name,
                "version": r.version,
                "status": r.status,
                "last_invoked": r.last_invoked,
            }),
        ),
        None => err(id, "E_NOT_FOUND", "function not found"),
    }
}

async fn list(state: Arc<AppState>, id: Value) -> Value {
    let fns = state.functions.lock().await;
    let items: Vec<Value> = fns
        .values()
        .map(|r| {
            json!({
                "name": r.name,
                "version": r.version,
                "status": r.status,
                "description": r.manifest.description,
            })
        })
        .collect();
    ok(id, json!(items))
}

async fn search(params: Option<Value>, state: Arc<AppState>, id: Value) -> Value {
    let p = params.unwrap_or(Value::Null);
    let want_desc = p
        .get("query")
        .and_then(|q| q.get("description"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());
    let fns = state.functions.lock().await;
    let mut items = Vec::new();
    for r in fns.values() {
        if let Some(w) = &want_desc {
            if !r.manifest.description.to_lowercase().contains(w) {
                continue;
            }
        }
        items.push(json!({
            "name": r.name,
            "version": r.version,
            "status": r.status,
            "description": r.manifest.description,
        }));
    }
    ok(id, json!({ "functions": items, "total": items.len() }))
}

async fn health(state: Arc<AppState>, id: Value) -> Value {
    let leases = state.leases.lock().await;
    let fns = state.functions.lock().await;
    ok(
        id,
        json!({
            "functions": fns.len(),
            "active_leases": leases.len(),
            "status": "healthy",
        }),
    )
}

async fn stop(params: Option<Value>, state: Arc<AppState>, id: Value) -> Value {
    let lid = match params
        .as_ref()
        .and_then(|p| p.get("lease_id"))
        .and_then(|v| v.as_str())
    {
        Some(l) => l.to_string(),
        None => return err(id, "E_INVALID", "lease_id required"),
    };
    let mut leases = state.leases.lock().await;
    match leases.remove(&lid) {
        Some(p) => {
            persistent_kill(&p);
            ok(id, json!({ "stopped": true }))
        }
        None => err(id, "E_NOT_FOUND", "unknown lease_id"),
    }
}

/// Parse string or string-array manifest fields.
fn parse_string_list(values: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    for v in values {
        if let Some(s) = v.as_str() {
            if !s.is_empty() {
                out.push(s.to_string());
            }
        }
    }
    out
}

/// Tell the MCP Bus to add a route for this lambda.
async fn register_bus_route(namespace: &str, lambda_name: &str, pattern: &str) {
    let path = std::env::var("THE_MACHINE_SOCKET_DIR")
        .map(|d| format!("{}/mcp-bus.sock", d))
        .unwrap_or_else(|_| "/run/the-machine/mcp-bus.sock".into());
    let req = json!({
        "id": 1,
        "kind": "Request",
        "method": "_bus.register",
        "params": {
            "namespace": namespace,
            "pattern": pattern,
            "handler": "lambda-server",
            "registered_by": "lambda-server",
            "manifest_ref": lambda_name,
        }
    });
    if let Ok(mut stream) = tokio::net::UnixStream::connect(&path).await {
        let mut buf = serde_json::to_vec(&req).unwrap_or_default();
        buf.push(b'\n');
        let _ = stream.write_all(&buf).await;
    }
}

/// Register an event handler with the Event Bus routing table.
async fn register_event_handler(lambda_name: &str, event_key: &str) {
    let (category, pattern) = match event_key.split_once('.') {
        Some((c, p)) => (c.to_string(), p.to_string()),
        None => (event_key.to_string(), "*".to_string()),
    };
    let path = std::env::var("THE_MACHINE_SOCKET_DIR")
        .map(|d| format!("{}/event-bus.sock", d))
        .unwrap_or_else(|_| "/run/the-machine/event-bus.sock".into());
    let req = json!({
        "id": 2,
        "kind": "Request",
        "method": "event.register_handler",
        "params": {
            "category": category,
            "pattern": pattern,
            "handler": "lambda-server",
            "manifest_ref": lambda_name,
        }
    });
    if let Ok(mut stream) = tokio::net::UnixStream::connect(&path).await {
        let mut buf = serde_json::to_vec(&req).unwrap_or_default();
        buf.push(b'\n');
        let _ = stream.write_all(&buf).await;
    }
}
