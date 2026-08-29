use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;
use tracing::{info, error, debug, warn};
use common::*;

mod registry;
use registry::{infer_namespace, Namespace, Registry, RouteEntry};

#[derive(Clone)]
struct AppState {
    registry: Arc<Mutex<Registry>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
        )
        .init();

    info!("MCP Bus starting...");

    token::init_token_secret();

    let state = AppState {
        registry: Arc::new(Mutex::new(Registry::new())),
    };

    // Pre-populate with system and state ops
    {
        let mut reg = state.registry.lock().await;
        reg.register("system-op:power.get_profile", "system-daemon", true).unwrap();
        reg.register("system-op:power.set_profile", "system-daemon", true).unwrap();
        reg.register("system-op:display.get_modes", "system-daemon", true).unwrap();
        reg.register("system-op:display.set_mode", "system-daemon", true).unwrap();
        reg.register("system-op:net.list_interfaces", "system-daemon", true).unwrap();
        reg.register("system-op:net.set_interface_state", "system-daemon", true).unwrap();
        reg.register("system-op:system-daemon.stats", "system-daemon", true).unwrap();
        reg.register("power.get_profile", "system-daemon", true).unwrap();
        reg.register("power.set_profile", "system-daemon", true).unwrap();
        reg.register("display.get_modes", "system-daemon", true).unwrap();
        reg.register("display.set_mode", "system-daemon", true).unwrap();
        reg.register("net.list_interfaces", "system-daemon", true).unwrap();
        reg.register("net.set_interface_state", "system-daemon", true).unwrap();
        reg.register("state-op:state.get", "state-store", true).unwrap();
        reg.register("state-op:state.set", "state-store", true).unwrap();
        reg.register("state-op:state.patch", "state-store", true).unwrap();
        reg.register("state-op:state.watch", "state-store", true).unwrap();
        reg.register("state.get", "state-store", true).unwrap();
        reg.register("state.set", "state-store", true).unwrap();
        reg.register("state.patch", "state-store", true).unwrap();
        reg.register("state.watch", "state-store", true).unwrap();
        reg.register("state.list", "state-store", true).unwrap();
        reg.register("state.get_revision", "state-store", true).unwrap();
        reg.register("state.stats", "state-store", true).unwrap();

        for m in [
            "event.publish",
            "event.emit",
            "event.subscribe",
            "event.unsubscribe",
            "event.register_handler",
            "event.schedule",
            "event.cancel",
            "event.stats",
            "event.explain_routing",
            "event.list_handlers",
            "event.list_agent_wakes",
            "bus.list_handlers",
            "bus.explain_routing",
        ] {
            reg.register(m, "event-bus", true).unwrap();
        }
        reg.register("bus.resolve", "mcp-bus", true).unwrap();
        reg.register("bus.list_routes", "mcp-bus", true).unwrap();
        reg.register("_bus.register", "mcp-bus", true).unwrap();
        reg.register("_bus.deregister", "mcp-bus", true).unwrap();

        for m in [
            "agent.status",
            "agent.interrupt",
            "agent.local_only_mode",
            "agent.skills.list",
        ] {
            reg.register(m, "agent-core", true).unwrap();
        }

        for m in [
            "lambda.invoke",
            "lambda.register",
            "lambda.deprecate",
            "lambda.lease",
            "lambda.list",
            "lambda.health",
            "lambda.stop",
            "lambda.search",
        ] {
            reg.register(m, "lambda-server", true).unwrap();
        }

        for m in [
            "policy.check",
            "policy.grant",
            "policy.revoke",
            "policy.audit",
            "policy.list",
            "policy.validate_register",
            "policy.confirm",
            "policy.confirm_result",
            "systemd.stop",
            "systemd.restart",
            "systemd.disable",
        ] {
            reg.register(m, "policy-broker", true).unwrap();
        }

        for m in ["ui.patch", "ui.get", "ui.bind", "ui.tree", "ui.event", "ui.status"] {
            reg.register(m, "ui-runtime", true).unwrap();
        }
        for m in ["compositor.surface", "compositor.blur", "compositor.focus"] {
            reg.register(m, "compositor", true).unwrap();
        }
    }

    // Rebuild dynamic routes persisted in State Store (Phase 3).
    reload_routes_from_state(&state).await;

    let socket_path = std::env::var("THE_MACHINE_SOCKET_DIR")
        .unwrap_or_else(|_| "/run/the-machine".to_string());
    let socket_path = format!("{}/mcp-bus.sock", socket_path);

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
                tokio::spawn(handle_client(stream, state));
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

async fn handle_client(mut stream: UnixStream, state: AppState) -> anyhow::Result<()> {
    debug!("Client connected");
    let mut buf = vec![0u8; 4096];
    loop {
        match stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                let data = &buf[..n];
                if let Some(response) = process_mcp_message(data, &state).await {
                    stream.write_all(&response).await?;
                }
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }
    debug!("Client disconnected");
    Ok(())
}

async fn process_mcp_message(data: &[u8], state: &AppState) -> Option<Vec<u8>> {
    let msg: serde_json::Value = match serde_json::from_slice(data) {
        Ok(v) => v,
        Err(_) => {
            error!("Invalid JSON: {}", String::from_utf8_lossy(data));
            return None;
        }
    };

    let method = match msg.get("method").and_then(|m| m.as_str()) {
        Some(m) => m,
        None => {
            error!("Missing method in request");
            return None;
        }
    };

    let id = msg.get("id").and_then(|i| i.as_u64()).unwrap_or(0);

    debug!("Handling method: {}", method);

    // Bus-local introspection and internal registration.
    if method == "bus.resolve"
        || method == "bus.list_routes"
        || method == "_bus.register"
        || method == "_bus.deregister"
    {
        return Some(handle_bus_local(method, &msg, state).await);
    }

    // Policy middleware: gate MCP calls before forwarding (Phase 3).
    if requires_policy_check(method) {
        let params = msg.get("params").cloned();
        if !policy_allows(method, params.as_ref()).await {
            return Some(error_bytes(id, "E_POLICY_DENY", "policy denied"));
        }
    }

    let reg = state.registry.lock().await;
    let resolved = reg.resolve_full(method);

    match resolved {
        Some(route) => {
            let handler_id = route.handler.clone();
            drop(reg);
            match forward_to(&handler_id, data, method, route.manifest_ref.as_deref()).await {
                Some(resp) => Some(resp),
                None => Some(error_bytes(id, "E_HANDLER_UNAVAILABLE",
                    &format!("Handler {} not reachable", handler_id))),
            }
        }
        None => {
            drop(reg);
            if let Some(handler_id) = prefix_handler(method) {
                match forward_to(&handler_id, data, method, None).await {
                    Some(resp) => Some(resp),
                    None => Some(error_bytes(id, "E_HANDLER_UNAVAILABLE",
                        &format!("Handler {} not reachable", handler_id))),
                }
            } else if method.starts_with("mcp-intent:") || infer_namespace(method) == Namespace::McpIntent {
                Some(error_bytes(id, "E_AGENT_REQUIRED",
                    "No handler registered, agent needed"))
            } else {
                Some(error_bytes(id, "E_NOT_FOUND",
                    &format!("No handler for method: {}", method)))
            }
        }
    }
}

async fn handle_bus_local(method: &str, msg: &serde_json::Value, state: &AppState) -> Vec<u8> {
    let id = msg.get("id").and_then(|i| i.as_u64()).unwrap_or(0);
    let params = msg.get("params").cloned().unwrap_or(serde_json::Value::Null);

    match method {
        "bus.resolve" => {
            let target = params
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let reg = state.registry.lock().await;
            match reg.resolve_full(target) {
                Some(r) => ok_bytes(id, serde_json::json!({
                    "method": target,
                    "namespace": r.namespace.as_str(),
                    "handler": r.handler,
                    "pattern": r.pattern,
                    "manifest_ref": r.manifest_ref,
                })),
                None => ok_bytes(id, serde_json::json!({
                    "method": target,
                    "decision": "AgentWake",
                    "reason": "NoMatch",
                })),
            }
        }
        "bus.list_routes" => {
            let ns = params
                .get("namespace")
                .and_then(|v| v.as_str())
                .and_then(registry::Namespace::from_str);
            let reg = state.registry.lock().await;
            let routes: Vec<serde_json::Value> = reg
                .list_routes(ns)
                .into_iter()
                .map(|e| serde_json::json!({
                    "namespace": e.namespace.as_str(),
                    "pattern": e.pattern,
                    "handler": e.handler,
                    "registered_by": e.registered_by,
                    "manifest_ref": e.manifest_ref,
                }))
                .collect();
            ok_bytes(id, serde_json::json!({ "routes": routes }))
        }
        "_bus.register" => {
            let registered_by = params
                .get("registered_by")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            // Only trusted infrastructure components may register routes.
            let allowed = matches!(
                registered_by,
                "lambda-server" | "event-bus" | "policy-broker" | "boot"
            );
            if !allowed {
                warn!("rejected _bus.register from {}", registered_by);
                return error_bytes(id, "E_FORBIDDEN", "registration not allowed");
            }
            let namespace = params
                .get("namespace")
                .and_then(|v| v.as_str())
                .and_then(registry::Namespace::from_str)
                .unwrap_or(Namespace::McpIntent);
            let pattern = match params.get("pattern").and_then(|v| v.as_str()) {
                Some(p) if !p.is_empty() => p.to_string(),
                _ => return error_bytes(id, "E_INVALID", "pattern required"),
            };
            let handler = match params.get("handler").and_then(|v| v.as_str()) {
                Some(h) => h.to_string(),
                _ => return error_bytes(id, "E_INVALID", "handler required"),
            };
            let manifest_ref = params
                .get("manifest_ref")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Broker validates registration (policy-broker-spec §11).
            if !validate_registration(&params).await {
                return error_bytes(id, "E_POLICY_DENY", "registration denied by policy broker");
            }

            let entry = RouteEntry {
                namespace,
                pattern,
                handler,
                registered_by: registered_by.to_string(),
                manifest_ref,
                trusted: true,
            };
            let mut reg = state.registry.lock().await;
            match reg.register_route(entry.clone()) {
                Ok(()) => {
                    persist_route(&entry).await;
                    ok_bytes(id, serde_json::json!({ "registered": true }))
                }
                Err(e) => error_bytes(id, "E_COLLISION", &e.to_string()),
            }
        }
        "_bus.deregister" => {
            let registered_by = params
                .get("registered_by")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let allowed = matches!(
                registered_by,
                "lambda-server" | "event-bus" | "policy-broker" | "boot"
            );
            if !allowed {
                return error_bytes(id, "E_FORBIDDEN", "deregistration not allowed");
            }
            let namespace = params
                .get("namespace")
                .and_then(|v| v.as_str())
                .and_then(registry::Namespace::from_str)
                .unwrap_or(Namespace::McpIntent);
            let pattern = match params.get("pattern").and_then(|v| v.as_str()) {
                Some(p) if !p.is_empty() => p.to_string(),
                _ => return error_bytes(id, "E_INVALID", "pattern required"),
            };
            let mut reg = state.registry.lock().await;
            let removed = reg.deregister_route(namespace, &pattern);
            if removed {
                delete_persisted_route(namespace, &pattern).await;
                ok_bytes(id, serde_json::json!({ "deregistered": true }))
            } else {
                error_bytes(id, "E_NOT_FOUND", "route not found")
            }
        }
        _ => error_bytes(id, "E_NOT_FOUND", "unknown bus method"),
    }
}

/// Forward `raw` to the handler component's socket. For lambda-hosted mcp-intent
/// calls, attach routing metadata so lambda-server can invoke the right function.
async fn forward_to(
    handler_id: &str,
    raw: &[u8],
    original_method: &str,
    manifest_ref: Option<&str>,
) -> Option<Vec<u8>> {
    let path = format!("/run/the-machine/{}.sock", handler_id);
    let mut stream = UnixStream::connect(&path).await.ok()?;

    let mut payload = raw.to_vec();
    // Inject routing hint for lambda-hosted intent proxies.
    if handler_id == "lambda-server" && manifest_ref.is_some() {
        if let Ok(mut v) = serde_json::from_slice::<serde_json::Value>(raw) {
            if let Some(params) = v.get_mut("params") {
                if params.is_null() {
                    *params = serde_json::json!({});
                }
                if let Some(obj) = params.as_object_mut() {
                    obj.insert(
                        "_route_method".into(),
                        serde_json::Value::String(original_method.into()),
                    );
                    if let Some(name) = manifest_ref {
                        obj.insert(
                            "_route_lambda".into(),
                            serde_json::Value::String(name.into()),
                        );
                    }
                }
            } else {
                v["params"] = serde_json::json!({
                    "_route_method": original_method,
                    "_route_lambda": manifest_ref,
                });
            }
            payload = serde_json::to_vec(&v).ok()?;
        }
    }

    let mut buf = payload;
    if !buf.ends_with(b"\n") {
        buf.push(b'\n');
    }
    stream.write_all(&buf).await.ok()?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let n = reader.read_line(&mut line).await.ok()?;
    if n == 0 {
        return None;
    }
    Some(line.into_bytes())
}

fn ok_bytes(id: u64, result: serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "id": id,
        "result": result
    }))
    .unwrap()
}

fn error_bytes(id: u64, code: &str, message: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "id": id,
        "error": { "code": code, "message": message }
    })).unwrap()
}

fn prefix_handler(method: &str) -> Option<String> {
    let prefix = method.split('.').next()?;
    let id = match prefix {
        "event" | "bus" => "event-bus",
        "lambda" => "lambda-server",
        "policy" => "policy-broker",
        "ui" => "ui-runtime",
        "compositor" => "compositor",
        "agent" => "agent-core",
        "state" => "state-store",
        "system" | "power" | "kernel" => "system-daemon",
        _ => return None,
    };
    Some(id.to_string())
}

/// Best-effort persist of a route to State Store (`perm.mcp_routes.*`).
async fn persist_route(entry: &RouteEntry) {
    let path = format!(
        "perm.mcp_routes.{}.{}",
        entry.namespace.as_str().replace('-', "_"),
        entry.pattern.replace('.', "_")
    );
    let value = serde_json::json!({
        "namespace": entry.namespace.as_str(),
        "pattern": entry.pattern,
        "handler": entry.handler,
        "registered_by": entry.registered_by,
        "manifest_ref": entry.manifest_ref,
    });
    let sock = std::env::var("THE_MACHINE_SOCKET_DIR")
        .map(|d| format!("{}/state-store.sock", d))
        .unwrap_or_else(|_| "/run/the-machine/state-store.sock".into());
    let req = serde_json::json!({
        "id": 0,
        "kind": "Request",
        "method": "state.set",
        "params": { "path": path, "value": value }
    });
    if let Ok(mut stream) = UnixStream::connect(&sock).await {
        let mut buf = serde_json::to_vec(&req).unwrap_or_default();
        buf.push(b'\n');
        let _ = stream.write_all(&buf).await;
    }
}

async fn delete_persisted_route(namespace: Namespace, pattern: &str) {
    let path = format!(
        "perm.mcp_routes.{}.{}",
        namespace.as_str().replace('-', "_"),
        pattern.replace('.', "_")
    );
    let sock = std::env::var("THE_MACHINE_SOCKET_DIR")
        .map(|d| format!("{}/state-store.sock", d))
        .unwrap_or_else(|_| "/run/the-machine/state-store.sock".into());
    let req = serde_json::json!({
        "id": 0,
        "kind": "Request",
        "method": "state.set",
        "params": { "path": path, "value": null }
    });
    if let Ok(mut stream) = UnixStream::connect(&sock).await {
        let mut buf = serde_json::to_vec(&req).unwrap_or_default();
        buf.push(b'\n');
        let _ = stream.write_all(&buf).await;
    }
}

async fn reload_routes_from_state(state: &AppState) {
    let sock = std::env::var("THE_MACHINE_SOCKET_DIR")
        .map(|d| format!("{}/state-store.sock", d))
        .unwrap_or_else(|_| "/run/the-machine/state-store.sock".into());
    let req = serde_json::json!({
        "id": 0,
        "kind": "Request",
        "method": "state.list",
        "params": { "prefix": "perm.mcp_routes." }
    });
    let Ok(mut stream) = UnixStream::connect(&sock).await else {
        return;
    };
    let mut buf = serde_json::to_vec(&req).unwrap_or_default();
    buf.push(b'\n');
    if stream.write_all(&buf).await.is_err() {
        return;
    }
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        return;
    }
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap_or(serde_json::Value::Null);
    let paths = resp
        .get("result")
        .and_then(|r| r.get("paths"))
        .and_then(|p| p.as_array());
    let Some(paths) = paths else {
        return;
    };
    let mut reg = state.registry.lock().await;
    let mut loaded = 0u64;
    for item in paths {
        let value = item.get("value").cloned().unwrap_or(serde_json::Value::Null);
        if value.is_null() {
            continue;
        }
        let namespace = value
            .get("namespace")
            .and_then(|v| v.as_str())
            .and_then(registry::Namespace::from_str)
            .unwrap_or(Namespace::McpIntent);
        let pattern = value
            .get("pattern")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let handler = value
            .get("handler")
            .and_then(|v| v.as_str())
            .unwrap_or("lambda-server")
            .to_string();
        let registered_by = value
            .get("registered_by")
            .and_then(|v| v.as_str())
            .unwrap_or("boot")
            .to_string();
        let manifest_ref = value
            .get("manifest_ref")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if pattern.is_empty() {
            continue;
        }
        let entry = RouteEntry {
            namespace,
            pattern,
            handler,
            registered_by,
            manifest_ref,
            trusted: true,
        };
        if reg.register_route(entry).is_ok() {
            loaded += 1;
        }
    }
    if loaded > 0 {
        info!("reloaded {} MCP routes from state store", loaded);
    }
}

fn requires_policy_check(method: &str) -> bool {
    if method.starts_with("policy.")
        || method.starts_with("bus.")
        || method == "_bus.register"
        || method == "_bus.deregister"
    {
        return false;
    }
    true
}

async fn policy_allows(method: &str, params: Option<&serde_json::Value>) -> bool {
    let capability = infer_capability(method);
    let path = params
        .and_then(|p| p.get("path"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| method.to_string());
    let principal = params
        .and_then(|p| p.get("principal"))
        .and_then(|v| v.as_str())
        .or_else(|| params.and_then(|p| p.get("registered_by")).and_then(|v| v.as_str()))
        .unwrap_or("mcp-bus");

    let sock = std::env::var("THE_MACHINE_SOCKET_DIR")
        .map(|d| format!("{}/policy-broker.sock", d))
        .unwrap_or_else(|_| "/run/the-machine/policy-broker.sock".into());
    let req = serde_json::json!({
        "id": 1,
        "kind": "Request",
        "method": "policy.check",
        "params": {
            "capability": capability,
            "path": path,
            "principal": principal,
            "method": method,
        }
    });
    let Ok(mut stream) = UnixStream::connect(&sock).await else {
        // If broker unavailable, fail open for boot path resilience.
        return true;
    };
    let mut buf = serde_json::to_vec(&req).unwrap_or_default();
    buf.push(b'\n');
    if stream.write_all(&buf).await.is_err() {
        return true;
    }
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        return true;
    }
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap_or(serde_json::Value::Null);
    resp.get("result")
        .and_then(|r| r.get("decision"))
        .and_then(|d| d.as_str())
        == Some("ALLOW")
}

async fn validate_registration(params: &serde_json::Value) -> bool {
    let sock = std::env::var("THE_MACHINE_SOCKET_DIR")
        .map(|d| format!("{}/policy-broker.sock", d))
        .unwrap_or_else(|_| "/run/the-machine/policy-broker.sock".into());
    let req = serde_json::json!({
        "id": 2,
        "kind": "Request",
        "method": "policy.validate_register",
        "params": params,
    });
    let Ok(mut stream) = UnixStream::connect(&sock).await else {
        return true;
    };
    let mut buf = serde_json::to_vec(&req).unwrap_or_default();
    buf.push(b'\n');
    if stream.write_all(&buf).await.is_err() {
        return true;
    }
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        return true;
    }
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap_or(serde_json::Value::Null);
    resp.get("result")
        .and_then(|r| r.get("allowed"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn infer_capability(method: &str) -> String {
    if method.starts_with("state.") {
        if matches!(method, "state.get" | "state.watch" | "state.list" | "state.get_revision" | "state.stats") {
            "CAP_STATE_READ".into()
        } else {
            "CAP_STATE_WRITE".into()
        }
    } else if method.starts_with("event.") || method.starts_with("bus.") {
        if method.contains("schedule") || method == "event.cancel" {
            "CAP_TIMER".into()
        } else if method.contains("register") || method.contains("subscribe") {
            "CAP_EVENT_ADMIN".into()
        } else {
            "CAP_EVENT_PUBLISH".into()
        }
    } else if method.starts_with("lambda.") {
        "CAP_IPC_CALL".into()
    } else {
        "CAP_IPC_CALL".into()
    }
}
