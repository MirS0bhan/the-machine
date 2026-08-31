use common::*;
use serde_json::json;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

mod auth;
mod external;
mod lease;
mod registry;
mod telemetry;
use auth::{authorize_deregister, authorize_register, RegisterAuthError};
use external::ExternalRegistry;
use lease::LeaseManager;
use registry::{infer_namespace, Namespace, Registry, RouteEntry};
use std::time::Instant;
use telemetry::Telemetry;

#[derive(Clone)]
struct AppState {
    registry: Arc<Mutex<Registry>>,
    leases: Arc<LeaseManager>,
    external: Arc<ExternalRegistry>,
    telemetry: Arc<Telemetry>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    info!("MCP Bus starting...");

    token::init_token_secret();

    let state = AppState {
        registry: Arc::new(Mutex::new(Registry::new())),
        leases: Arc::new(LeaseManager::new(300)),
        external: Arc::new(ExternalRegistry::new()),
        telemetry: Arc::new(Telemetry::new(10_000)),
    };

    // Pre-populate with system and state ops
    {
        let mut reg = state.registry.lock().await;
        reg.register("system-op:power.get_profile", "system-daemon", true)
            .unwrap();
        reg.register("system-op:power.set_profile", "system-daemon", true)
            .unwrap();
        reg.register("system-op:display.get_modes", "system-daemon", true)
            .unwrap();
        reg.register("system-op:display.set_mode", "system-daemon", true)
            .unwrap();
        reg.register("system-op:net.list_interfaces", "system-daemon", true)
            .unwrap();
        reg.register("system-op:net.set_interface_state", "system-daemon", true)
            .unwrap();
        reg.register("system-op:system-daemon.stats", "system-daemon", true)
            .unwrap();
        for m in [
            "power.get_profile",
            "power.set_profile",
            "display.get_modes",
            "display.set_mode",
            "net.list_interfaces",
            "net.set_interface_state",
            "net.get_wifi_status",
            "net.connect_wifi",
            "audio.list_devices",
            "audio.set_default",
            "clipboard.get",
            "clipboard.set",
        ] {
            reg.register(m, "system-daemon", true).unwrap();
        }
        reg.register("state-op:state.get", "state-store", true)
            .unwrap();
        reg.register("state-op:state.set", "state-store", true)
            .unwrap();
        reg.register("state-op:state.patch", "state-store", true)
            .unwrap();
        reg.register("state-op:state.watch", "state-store", true)
            .unwrap();
        reg.register("state.get", "state-store", true).unwrap();
        reg.register("state.set", "state-store", true).unwrap();
        reg.register("state.patch", "state-store", true).unwrap();
        reg.register("state.watch", "state-store", true).unwrap();
        reg.register("state.list", "state-store", true).unwrap();
        reg.register("state.get_revision", "state-store", true)
            .unwrap();
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
            "agent.cloud.status",
            "agent.cloud.reload",
            "agent.interrupt",
            "agent.local_only_mode",
            "agent.skills.list",
            "agent.skills.reload",
            "agent.chat.send",
            "agent.chat.history",
            "agent.chat.export",
            "agent.chat.suggest",
            "agent.chat.pin",
            "agent.chat.clear",
            "agent.chat.undo",
            "agent.chat.edit",
            "agent.chat.regenerate",
            "agent.tour.next",
            "agent.desktop.spawn",
            "agent.desktop.clear",
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
            "policy.register",
            "policy.grant",
            "policy.revoke",
            "policy.audit",
            "policy.list",
            "policy.validate_register",
            "policy.confirm",
            "policy.confirm_result",
            "policy.confirmation.pending",
            "systemd.stop",
            "systemd.restart",
            "systemd.disable",
        ] {
            reg.register(m, "policy-broker", true).unwrap();
        }

        for m in [
            "ui.patch",
            "ui.get",
            "ui.bind",
            "ui.tree",
            "ui.event",
            "ui.status",
            "ui.focus.get",
            "ui.focus.set",
            "ui.focus.next",
            "ui.theme.get",
            "ui.theme.set",
            "ui.auil.parse",
            "ui.auil.load",
            "ui.a11y.tree",
            "ui.atspi.status",
            "ui.i18n.status",
            "ui.i18n.t",
            "ui.i18n.load",
            "ui.i18n.locales",
            "ui.components.list",
            "ui.workspace.clear",
            "ui.workspace.replace",
            "ui.workspace.list",
            "ui.shortcuts.list",
            "ui.shortcuts.set",
            "ui.shortcuts.reset",
            "ui.a11y.announce",
            "ui.a11y.focus_order",
            "ui.select.all",
            "ui.scroll",
            "ui.snapshot",
            "ui.menu.open",
        ] {
            reg.register(m, "ui-runtime", true).unwrap();
        }
        for m in [
            "compositor.surface",
            "compositor.blur",
            "compositor.focus",
            "compositor.input",
            "compositor.present",
            "compositor.list",
            "compositor.status",
            "compositor.confirmation.set_active",
        ] {
            reg.register(m, "compositor", true).unwrap();
        }
        for m in [
            "localmodel.complete",
            "localmodel.embed",
            "localmodel.classify_intent",
            "localmodel.health",
        ] {
            reg.register(m, "local-model-daemon", true).unwrap();
        }
        for m in [
            "marketplace.list",
            "marketplace.install",
            "marketplace.installed",
        ] {
            reg.register(m, "marketplace", true).unwrap();
        }
        for m in ["shell.status", "shell.activate", "hello"] {
            reg.register(m, "fallback-shell", true).unwrap();
        }
        reg.register("bus.lease", "mcp-bus", true).unwrap();
        reg.register("bus.lease.renew", "mcp-bus", true).unwrap();
        reg.register("bus.telemetry.export", "mcp-bus", true)
            .unwrap();
        reg.register("bus.external.register", "mcp-bus", true)
            .unwrap();
        reg.register("bus.external.list", "mcp-bus", true).unwrap();
        reg.register("bus.external.forward", "mcp-bus", true)
            .unwrap();
    }

    // Rebuild dynamic routes persisted in State Store (Phase 3).
    reload_routes_from_state(&state).await;

    let socket_path = common::bus_socket();

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

async fn handle_client(stream: UnixStream, state: AppState) -> anyhow::Result<()> {
    debug!("Client connected");
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
                if let Some(response) = process_mcp_message(trimmed.as_bytes(), &state).await {
                    writer.write_all(&response).await?;
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
        || method.starts_with("bus.lease")
        || method.starts_with("bus.telemetry")
        || method.starts_with("bus.external")
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
            match forward_to(
                &handler_id,
                data,
                method,
                route.manifest_ref.as_deref(),
                state,
            )
            .await
            {
                Some(resp) => Some(resp),
                None => Some(error_bytes(
                    id,
                    "E_HANDLER_UNAVAILABLE",
                    &format!("Handler {} not reachable", handler_id),
                )),
            }
        }
        None => {
            drop(reg);
            if let Some(handler_id) = prefix_handler(method) {
                match forward_to(&handler_id, data, method, None, state).await {
                    Some(resp) => Some(resp),
                    None => Some(error_bytes(
                        id,
                        "E_HANDLER_UNAVAILABLE",
                        &format!("Handler {} not reachable", handler_id),
                    )),
                }
            } else if method.starts_with("mcp-intent:")
                || infer_namespace(method) == Namespace::McpIntent
            {
                Some(error_bytes(
                    id,
                    "E_AGENT_REQUIRED",
                    "No handler registered, agent needed",
                ))
            } else {
                Some(error_bytes(
                    id,
                    "E_NOT_FOUND",
                    &format!("No handler for method: {}", method),
                ))
            }
        }
    }
}

async fn handle_bus_local(method: &str, msg: &serde_json::Value, state: &AppState) -> Vec<u8> {
    let id = msg.get("id").and_then(|i| i.as_u64()).unwrap_or(0);
    let params = msg
        .get("params")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    match method {
        "bus.resolve" => {
            let target = params.get("method").and_then(|v| v.as_str()).unwrap_or("");
            let reg = state.registry.lock().await;
            match reg.resolve_full(target) {
                Some(r) => ok_bytes(
                    id,
                    serde_json::json!({
                        "method": target,
                        "namespace": r.namespace.as_str(),
                        "handler": r.handler,
                        "pattern": r.pattern,
                        "manifest_ref": r.manifest_ref,
                    }),
                ),
                None => ok_bytes(
                    id,
                    serde_json::json!({
                        "method": target,
                        "decision": "AgentWake",
                        "reason": "NoMatch",
                    }),
                ),
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
                .map(|e| {
                    serde_json::json!({
                        "namespace": e.namespace.as_str(),
                        "pattern": e.pattern,
                        "handler": e.handler,
                        "registered_by": e.registered_by,
                        "manifest_ref": e.manifest_ref,
                        "trusted": e.trusted,
                    })
                })
                .collect();
            ok_bytes(id, serde_json::json!({ "routes": routes }))
        }
        "_bus.register" => {
            let registered_by = params
                .get("registered_by")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
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
            match authorize_register(registered_by, &handler, namespace) {
                Ok(()) => {}
                Err(RegisterAuthError::Forbidden) => {
                    warn!("rejected _bus.register from {}", registered_by);
                    return error_bytes(id, "E_FORBIDDEN", "registration not allowed");
                }
                Err(RegisterAuthError::InvalidNamespace) => {
                    return error_bytes(
                        id,
                        "E_INVALID_NAMESPACE",
                        "runtime registration limited to mcp-intent and event-handler",
                    );
                }
                Err(RegisterAuthError::HandlerMismatch) => {
                    warn!(
                        "rejected _bus.register: {} cannot register handler {}",
                        registered_by, handler
                    );
                    return error_bytes(id, "E_FORBIDDEN", "handler must match registrar identity");
                }
            }
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
            if let Err(RegisterAuthError::Forbidden) = authorize_deregister(registered_by) {
                warn!("rejected _bus.deregister from {}", registered_by);
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
        "bus.lease" => {
            state.leases.purge_expired();
            let method_name = params.get("method").and_then(|v| v.as_str()).unwrap_or("");
            let reg = state.registry.lock().await;
            let route = match reg.resolve_full(method_name) {
                Some(r) => r,
                None => return error_bytes(id, "E_NOT_FOUND", "method not registered"),
            };
            let ttl = params.get("ttl_secs").and_then(|v| v.as_u64());
            let mut result =
                state
                    .leases
                    .create(method_name, &route.handler, route.manifest_ref.clone(), ttl);
            if lease_fast_path_enabled() {
                if let Some(lease_id) = result.get("lease_id").and_then(|v| v.as_str()) {
                    if let Some(rec) = state.leases.get(lease_id) {
                        match spawn_lease_relay(
                            rec.lease_id.clone(),
                            rec.method.clone(),
                            rec.handler.clone(),
                            rec.manifest_ref.clone(),
                            rec.expires_at,
                            state.clone(),
                        )
                        .await
                        {
                            Ok(socket_path) => {
                                if let Some(obj) = result.as_object_mut() {
                                    obj.insert("fast_path".into(), json!(true));
                                    obj.insert("socket_path".into(), json!(socket_path));
                                }
                            }
                            Err(e) => {
                                warn!("lease fast-path bind failed for {}: {}", lease_id, e);
                            }
                        }
                    }
                }
            }
            ok_bytes(id, result)
        }
        "bus.lease.renew" => {
            state.leases.purge_expired();
            let lease_id = params
                .get("lease_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let ttl = params.get("ttl_secs").and_then(|v| v.as_u64());
            match state.leases.renew(lease_id, ttl) {
                Some(r) => ok_bytes(id, r),
                None => error_bytes(id, "E_NOT_FOUND", "lease not found or expired"),
            }
        }
        "bus.telemetry.export" => {
            let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;
            ok_bytes(id, state.telemetry.export(limit))
        }
        "bus.external.register" => {
            if !policy_allows("bus.external.register", Some(&params)).await {
                return error_bytes(id, "E_POLICY_DENY", "policy denied");
            }
            let server_id = params.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let base_url = params
                .get("base_url")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let methods: Vec<String> = params
                .get("allowed_methods")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            match state.external.register(server_id, base_url, methods) {
                Ok(result) => ok_bytes(id, result),
                Err(e) => error_bytes(id, "E_INVALID", &e),
            }
        }
        "bus.external.list" => ok_bytes(id, state.external.list()),
        "bus.external.forward" => {
            if !policy_allows("bus.external.forward", Some(&params)).await {
                return error_bytes(id, "E_POLICY_DENY", "policy denied");
            }
            let server_id = params
                .get("server_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let fwd_method = params
                .get("forward_method")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let payload = params
                .get("payload")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let result = state.external.forward(server_id, fwd_method, payload).await;
            match result {
                Some(r) => ok_bytes(id, r),
                None => error_bytes(id, "E_NOT_FOUND", "external server unavailable"),
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
    state: &AppState,
) -> Option<Vec<u8>> {
    let started = Instant::now();
    let path = common::component_socket(handler_id);
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
    let ok = !line.contains("\"error\"");
    state.telemetry.record(
        original_method,
        handler_id,
        started.elapsed().as_millis() as u64,
        ok,
    );
    Some(line.into_bytes())
}

fn lease_fast_path_enabled() -> bool {
    matches!(
        std::env::var("THE_MACHINE_LEASE_FAST_PATH").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

/// Bind `leases/<id>.sock` and relay the leased method to its handler (G12).
async fn spawn_lease_relay(
    lease_id: String,
    method: String,
    handler: String,
    manifest_ref: Option<String>,
    expires_at: std::time::Instant,
    state: AppState,
) -> anyhow::Result<String> {
    let socket_path = common::lease_socket(&lease_id);
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let _ = tokio::fs::remove_file(&socket_path).await;
    let listener = UnixListener::bind(&socket_path)?;
    let cleanup_path = socket_path.clone();

    tokio::spawn(async move {
        loop {
            if std::time::Instant::now() >= expires_at {
                debug!("lease {} expired, stopping relay", lease_id);
                break;
            }
            let accept =
                tokio::time::timeout(std::time::Duration::from_millis(500), listener.accept())
                    .await;
            match accept {
                Ok(Ok((stream, _))) => {
                    let state = state.clone();
                    let method = method.clone();
                    let handler = handler.clone();
                    let manifest_ref = manifest_ref.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_lease_relay_client(
                            stream,
                            &method,
                            &handler,
                            manifest_ref.as_deref(),
                            &state,
                        )
                        .await
                        {
                            warn!("lease relay client error: {}", e);
                        }
                    });
                }
                Ok(Err(e)) => {
                    error!("lease relay accept failed: {}", e);
                    break;
                }
                Err(_) => continue,
            }
        }
        let _ = tokio::fs::remove_file(&cleanup_path).await;
    });

    Ok(socket_path)
}

async fn handle_lease_relay_client(
    stream: UnixStream,
    leased_method: &str,
    handler: &str,
    manifest_ref: Option<&str>,
    state: &AppState,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    line.clear();
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        return Ok(());
    }
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let msg: serde_json::Value = serde_json::from_str(trimmed)?;
    let method = msg
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or(leased_method);
    if method != leased_method {
        let id = msg.get("id").and_then(|i| i.as_u64()).unwrap_or(0);
        writer
            .write_all(&error_bytes(id, "E_INVALID", "method does not match lease"))
            .await?;
        return Ok(());
    }

    let id = msg.get("id").and_then(|i| i.as_u64()).unwrap_or(0);
    let params = msg.get("params").cloned();
    if requires_policy_check(method) && !policy_allows(method, params.as_ref()).await {
        writer
            .write_all(&error_bytes(id, "E_POLICY_DENY", "policy denied"))
            .await?;
        return Ok(());
    }

    match forward_to(handler, trimmed.as_bytes(), method, manifest_ref, state).await {
        Some(resp) => {
            writer.write_all(&resp).await?;
        }
        None => {
            writer
                .write_all(&error_bytes(
                    id,
                    "E_HANDLER_UNAVAILABLE",
                    &format!("Handler {} not reachable", handler),
                ))
                .await?;
        }
    }
    Ok(())
}

fn json_line(value: serde_json::Value) -> Vec<u8> {
    let mut buf = serde_json::to_vec(&value).unwrap();
    buf.push(b'\n');
    buf
}

fn ok_bytes(id: u64, result: serde_json::Value) -> Vec<u8> {
    json_line(serde_json::json!({
        "id": id,
        "result": result
    }))
}

fn error_bytes(id: u64, code: &str, message: &str) -> Vec<u8> {
    json_line(serde_json::json!({
        "id": id,
        "error": { "code": code, "message": message }
    }))
}

fn prefix_handler(method: &str) -> Option<String> {
    let prefix = method.split('.').next()?;
    let id = match prefix {
        "event" | "bus" => "event-bus",
        "lambda" => "lambda-server",
        "policy" | "systemd" => "policy-broker",
        "ui" => "ui-runtime",
        "compositor" => "compositor",
        "agent" => "agent-core",
        "state" => "state-store",
        "system" | "power" | "kernel" | "display" | "net" | "audio" => "system-daemon",
        "localmodel" => "local-model-daemon",
        "marketplace" => "marketplace",
        "shell" | "hello" => "fallback-shell",
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
    let sock = common::component_socket("state-store");
    let req = serde_json::json!({
        "id": Uuid::new_v4(),
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
    let sock = common::component_socket("state-store");
    let req = serde_json::json!({
        "id": Uuid::new_v4(),
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
    let sock = common::component_socket("state-store");
    let req = serde_json::json!({
        "id": Uuid::new_v4(),
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
        let value = item
            .get("value")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
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

fn policy_fail_open_all() -> bool {
    matches!(
        std::env::var("THE_MACHINE_POLICY_FAIL_OPEN").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

/// When the broker is down, only documented read-only / boot-health methods
/// proceed. Mutations and registration fail closed unless
/// `THE_MACHINE_POLICY_FAIL_OPEN=1` (dev override).
fn policy_unavailable_allows(method: &str) -> bool {
    policy_fail_open_all() || is_boot_readonly(method)
}

fn is_boot_readonly(method: &str) -> bool {
    matches!(
        method,
        "state.get"
            | "state.watch"
            | "state.list"
            | "state.get_revision"
            | "state.stats"
            | "power.get_profile"
            | "display.get_modes"
            | "net.list_interfaces"
            | "net.get_wifi_status"
            | "audio.list_devices"
            | "system-daemon.stats"
            | "clipboard.get"
            | "agent.status"
            | "lambda.list"
            | "lambda.health"
            | "ui.get"
            | "ui.tree"
            | "ui.status"
            | "ui.a11y.tree"
            | "ui.atspi.status"
            | "ui.i18n.status"
            | "ui.i18n.t"
            | "ui.components.list"
            | "compositor.status"
            | "compositor.list"
            | "event.stats"
            | "event.list_handlers"
            | "localmodel.health"
            | "shell.status"
            | "hello"
            | "marketplace.list"
            | "marketplace.installed"
    ) || method.ends_with(".health")
        || method.ends_with(".status")
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
    if policy_fail_open_all() {
        return true;
    }
    let capability = infer_capability(method);
    let path = params
        .and_then(|p| p.get("path"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| method.to_string());
    let principal = params
        .and_then(|p| p.get("principal"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            params
                .and_then(|p| p.get("registered_by"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("mcp-bus");

    let sock = common::component_socket("policy-broker");
    let req = serde_json::json!({
        "id": Uuid::new_v4(),
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
        return policy_unavailable_allows(method);
    };
    let mut buf = serde_json::to_vec(&req).unwrap_or_default();
    buf.push(b'\n');
    if stream.write_all(&buf).await.is_err() {
        return policy_unavailable_allows(method);
    }
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        return policy_unavailable_allows(method);
    }
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap_or(serde_json::Value::Null);
    resp.get("result")
        .and_then(|r| r.get("decision"))
        .and_then(|d| d.as_str())
        == Some("ALLOW")
}

async fn validate_registration(params: &serde_json::Value) -> bool {
    let sock = common::component_socket("policy-broker");
    let req = serde_json::json!({
        "id": Uuid::new_v4(),
        "kind": "Request",
        "method": "policy.validate_register",
        "params": params,
    });
    let Ok(mut stream) = UnixStream::connect(&sock).await else {
        return policy_fail_open_all();
    };
    let mut buf = serde_json::to_vec(&req).unwrap_or_default();
    buf.push(b'\n');
    if stream.write_all(&buf).await.is_err() {
        return policy_fail_open_all();
    }
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        return policy_fail_open_all();
    }
    let resp: serde_json::Value = serde_json::from_str(&line).unwrap_or(serde_json::Value::Null);
    resp.get("result")
        .and_then(|r| r.get("allowed"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn infer_capability(method: &str) -> String {
    if method.starts_with("state.") {
        if matches!(
            method,
            "state.get" | "state.watch" | "state.list" | "state.get_revision" | "state.stats"
        ) {
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
    } else {
        "CAP_IPC_CALL".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responses_are_newline_delimited() {
        let ok = ok_bytes(1, serde_json::json!({"ok": true}));
        assert!(ok.ends_with(b"\n"), "ok_bytes must terminate with newline");
        let err = error_bytes(1, "E_TEST", "nope");
        assert!(
            err.ends_with(b"\n"),
            "error_bytes must terminate with newline"
        );
        let v: serde_json::Value = serde_json::from_slice(ok.trim_ascii()).unwrap();
        assert_eq!(v["result"]["ok"], true);
    }

    #[test]
    fn prefix_handler_covers_phase7_services() {
        assert_eq!(
            prefix_handler("localmodel.complete").as_deref(),
            Some("local-model-daemon")
        );
        assert_eq!(
            prefix_handler("marketplace.list").as_deref(),
            Some("marketplace")
        );
        assert_eq!(
            prefix_handler("display.get_modes").as_deref(),
            Some("system-daemon")
        );
        assert_eq!(
            prefix_handler("net.list_interfaces").as_deref(),
            Some("system-daemon")
        );
        assert_eq!(
            prefix_handler("audio.list_devices").as_deref(),
            Some("system-daemon")
        );
        assert_eq!(
            prefix_handler("systemd.stop").as_deref(),
            Some("policy-broker")
        );
        assert_eq!(
            prefix_handler("shell.status").as_deref(),
            Some("fallback-shell")
        );
        assert_eq!(prefix_handler("hello").as_deref(), Some("fallback-shell"));
    }

    #[test]
    fn lease_fast_path_env_toggle() {
        std::env::set_var("THE_MACHINE_LEASE_FAST_PATH", "1");
        assert!(lease_fast_path_enabled());
        std::env::set_var("THE_MACHINE_LEASE_FAST_PATH", "true");
        assert!(lease_fast_path_enabled());
        std::env::remove_var("THE_MACHINE_LEASE_FAST_PATH");
        assert!(!lease_fast_path_enabled());
    }

    #[test]
    fn broker_down_fails_closed_for_mutations() {
        std::env::remove_var("THE_MACHINE_POLICY_FAIL_OPEN");
        assert!(is_boot_readonly("state.get"));
        assert!(is_boot_readonly("shell.status"));
        assert!(!is_boot_readonly("state.set"));
        assert!(!is_boot_readonly("power.set_profile"));
        assert!(!is_boot_readonly("lambda.register"));
        assert!(!is_boot_readonly("bus.external.register"));
        assert!(!is_boot_readonly("bus.external.forward"));
        assert!(policy_unavailable_allows("ui.status"));
        assert!(!policy_unavailable_allows("net.connect_wifi"));
    }

    async fn test_state() -> AppState {
        let state = AppState {
            registry: Arc::new(Mutex::new(Registry::new())),
            leases: Arc::new(LeaseManager::new(300)),
            external: Arc::new(ExternalRegistry::new()),
            telemetry: Arc::new(Telemetry::new(10_000)),
        };
        {
            let mut reg = state.registry.lock().await;
            reg.register("state.get", "state-store", true).unwrap();
            reg.register("power.get_profile", "system-daemon", true)
                .unwrap();
        }
        state
    }

    fn bus_msg(id: u64, params: serde_json::Value) -> serde_json::Value {
        json!({ "id": id, "params": params })
    }

    fn parse_resp(bytes: &[u8]) -> serde_json::Value {
        serde_json::from_slice(bytes.trim_ascii()).unwrap()
    }

    #[tokio::test]
    async fn bus_resolve_known_route_returns_handler() {
        let state = test_state().await;
        let resp = handle_bus_local(
            "bus.resolve",
            &bus_msg(1, json!({ "method": "state.get" })),
            &state,
        )
        .await;
        let v = parse_resp(&resp);
        assert!(v.get("error").is_none());
        assert_eq!(
            v.get("result")
                .and_then(|r| r.get("handler"))
                .and_then(|h| h.as_str()),
            Some("state-store")
        );
    }

    #[tokio::test]
    async fn bus_resolve_unknown_returns_agent_wake() {
        let state = test_state().await;
        let resp = handle_bus_local(
            "bus.resolve",
            &bus_msg(2, json!({ "method": "calc.add" })),
            &state,
        )
        .await;
        let v = parse_resp(&resp);
        assert!(v.get("error").is_none());
        assert_eq!(
            v.get("result")
                .and_then(|r| r.get("decision"))
                .and_then(|d| d.as_str()),
            Some("AgentWake")
        );
    }

    #[tokio::test]
    async fn bus_list_routes_includes_seeded_routes() {
        let state = test_state().await;
        let resp = handle_bus_local("bus.list_routes", &bus_msg(3, json!({})), &state).await;
        let v = parse_resp(&resp);
        assert!(v.get("error").is_none());
        let routes = v
            .get("result")
            .and_then(|r| r.get("routes"))
            .and_then(|r| r.as_array())
            .expect("routes array");
        assert!(routes.len() >= 2);
        let patterns: Vec<&str> = routes
            .iter()
            .filter_map(|e| e.get("pattern").and_then(|p| p.as_str()))
            .collect();
        assert!(patterns.contains(&"state.get"));
        assert!(patterns.contains(&"power.get_profile"));
    }

    #[tokio::test]
    async fn bus_register_rejects_spoofed_boot_registrar() {
        let state = test_state().await;
        let resp = handle_bus_local(
            "_bus.register",
            &bus_msg(
                4,
                json!({
                    "registered_by": "boot",
                    "namespace": "mcp-intent",
                    "pattern": "calc.add",
                    "handler": "lambda-server"
                }),
            ),
            &state,
        )
        .await;
        let v = parse_resp(&resp);
        assert_eq!(
            v.get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_str()),
            Some("E_FORBIDDEN")
        );
    }

    #[tokio::test]
    async fn bus_register_rejects_missing_pattern() {
        let state = test_state().await;
        let resp = handle_bus_local(
            "_bus.register",
            &bus_msg(
                5,
                json!({
                    "registered_by": "lambda-server",
                    "namespace": "mcp-intent",
                    "handler": "lambda-server"
                }),
            ),
            &state,
        )
        .await;
        let v = parse_resp(&resp);
        assert_eq!(
            v.get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_str()),
            Some("E_INVALID")
        );
    }

    #[tokio::test]
    async fn bus_register_and_deregister_roundtrip() {
        std::env::set_var("THE_MACHINE_POLICY_FAIL_OPEN", "1");
        let state = test_state().await;
        let pattern = "test.maintenance.echo";

        let reg_resp = handle_bus_local(
            "_bus.register",
            &bus_msg(
                6,
                json!({
                    "registered_by": "lambda-server",
                    "namespace": "mcp-intent",
                    "pattern": pattern,
                    "handler": "lambda-server"
                }),
            ),
            &state,
        )
        .await;
        let reg = parse_resp(&reg_resp);
        assert!(reg.get("error").is_none());
        assert_eq!(
            reg.get("result")
                .and_then(|r| r.get("registered"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );

        let resolve = handle_bus_local(
            "bus.resolve",
            &bus_msg(7, json!({ "method": pattern })),
            &state,
        )
        .await;
        let resolved = parse_resp(&resolve);
        assert_eq!(
            resolved
                .get("result")
                .and_then(|r| r.get("handler"))
                .and_then(|h| h.as_str()),
            Some("lambda-server")
        );

        let dereg_resp = handle_bus_local(
            "_bus.deregister",
            &bus_msg(
                8,
                json!({
                    "registered_by": "lambda-server",
                    "namespace": "mcp-intent",
                    "pattern": pattern
                }),
            ),
            &state,
        )
        .await;
        let dereg = parse_resp(&dereg_resp);
        assert!(dereg.get("error").is_none());
        assert_eq!(
            dereg
                .get("result")
                .and_then(|r| r.get("deregistered"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        std::env::remove_var("THE_MACHINE_POLICY_FAIL_OPEN");
    }

    #[tokio::test]
    async fn bus_lease_creates_lease_for_registered_method() {
        let state = test_state().await;
        let resp = handle_bus_local(
            "bus.lease",
            &bus_msg(9, json!({ "method": "state.get", "ttl_secs": 60 })),
            &state,
        )
        .await;
        let v = parse_resp(&resp);
        assert!(v.get("error").is_none());
        let result = v.get("result").expect("result");
        assert!(result.get("lease_id").and_then(|v| v.as_str()).is_some());
        assert_eq!(
            result.get("handler").and_then(|v| v.as_str()),
            Some("state-store")
        );
    }

    #[tokio::test]
    async fn bus_lease_unknown_method_returns_not_found() {
        let state = test_state().await;
        let resp = handle_bus_local(
            "bus.lease",
            &bus_msg(10, json!({ "method": "missing.method" })),
            &state,
        )
        .await;
        let v = parse_resp(&resp);
        assert_eq!(
            v.get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_str()),
            Some("E_NOT_FOUND")
        );
    }

    #[tokio::test]
    async fn bus_lease_renew_unknown_returns_not_found() {
        let state = test_state().await;
        let resp = handle_bus_local(
            "bus.lease.renew",
            &bus_msg(11, json!({ "lease_id": "no-such-lease" })),
            &state,
        )
        .await;
        let v = parse_resp(&resp);
        assert_eq!(
            v.get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_str()),
            Some("E_NOT_FOUND")
        );
    }

    #[tokio::test]
    async fn bus_external_list_empty_by_default() {
        let state = test_state().await;
        let resp = handle_bus_local("bus.external.list", &bus_msg(12, json!({})), &state).await;
        let v = parse_resp(&resp);
        assert!(v.get("error").is_none());
        let servers = v
            .get("result")
            .and_then(|r| r.get("servers"))
            .and_then(|s| s.as_array())
            .expect("servers array");
        assert!(servers.is_empty());
    }

    #[tokio::test]
    async fn bus_external_register_rejects_open_proxy_url() {
        std::env::set_var("THE_MACHINE_POLICY_FAIL_OPEN", "1");
        let state = test_state().await;
        let resp = handle_bus_local(
            "bus.external.register",
            &bus_msg(
                13,
                json!({
                    "id": "evil",
                    "base_url": "http://169.254.169.254/",
                    "allowed_methods": ["fetch"]
                }),
            ),
            &state,
        )
        .await;
        let v = parse_resp(&resp);
        assert_eq!(
            v.get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_str()),
            Some("E_INVALID")
        );
        std::env::remove_var("THE_MACHINE_POLICY_FAIL_OPEN");
    }

    #[tokio::test]
    async fn unknown_bus_method_returns_not_found() {
        let state = test_state().await;
        let resp = handle_bus_local("bus.nope", &bus_msg(14, json!({})), &state).await;
        let v = parse_resp(&resp);
        assert_eq!(
            v.get("error")
                .and_then(|e| e.get("code"))
                .and_then(|c| c.as_str()),
            Some("E_NOT_FOUND")
        );
    }
}
