use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt, AsyncBufReadExt, BufReader};
use tokio::sync::Mutex;
use tracing::{info, error, debug, warn};
use common::*;

mod registry;
use registry::Registry;

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
        // Bare names (as implemented by the service handlers) for direct routing.
        reg.register("state.get", "state-store", true).unwrap();
        reg.register("state.set", "state-store", true).unwrap();
        reg.register("state.patch", "state-store", true).unwrap();
        reg.register("state.watch", "state-store", true).unwrap();

        // Event/Scheduler Bus surface.
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

        // Agent Core surface.
        for m in [
            "agent.status",
            "agent.interrupt",
            "agent.local_only_mode",
        ] {
            reg.register(m, "agent-core", true).unwrap();
        }

        // Lambda Server surface.
        for m in [
            "lambda.invoke",
            "lambda.register",
            "lambda.lease",
            "lambda.list",
            "lambda.health",
            "lambda.stop",
        ] {
            reg.register(m, "lambda-server", true).unwrap();
        }

        // Policy Broker surface.
        for m in [
            "policy.check",
            "policy.grant",
            "policy.revoke",
            "policy.audit",
            "policy.list",
            "systemd.stop",
            "systemd.restart",
            "systemd.disable",
        ] {
            reg.register(m, "policy-broker", true).unwrap();
        }

        // UI Runtime + Compositor surface.
        for m in ["ui.patch", "ui.get", "ui.bind", "ui.tree"] {
            reg.register(m, "ui-runtime", true).unwrap();
        }
        for m in ["compositor.surface", "compositor.blur", "compositor.focus"] {
            reg.register(m, "compositor", true).unwrap();
        }
    }

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

    // Resolve method to handler
    let reg = state.registry.lock().await;
    let handler = reg.resolve(method);

    match handler {
        Some(handler_id) => {
            // Forward the raw request to the handler's Unix socket and relay its response.
            match forward_to(&handler_id, data).await {
                Some(resp) => Some(resp),
                None => Some(error_bytes(id, "E_HANDLER_UNAVAILABLE",
                    &format!("Handler {} not reachable", handler_id))),
            }
        }
        None => {
            // No static handler: try prefix-based routing (e.g. ui.* -> ui-runtime).
            if let Some(handler_id) = prefix_handler(method) {
                match forward_to(&handler_id, data).await {
                    Some(resp) => Some(resp),
                    None => Some(error_bytes(id, "E_HANDLER_UNAVAILABLE",
                        &format!("Handler {} not reachable", handler_id))),
                }
            } else if method.starts_with("mcp-intent:") {
                Some(error_bytes(id, "E_AGENT_REQUIRED",
                    "No handler registered, agent needed"))
            } else {
                Some(error_bytes(id, "E_NOT_FOUND",
                    &format!("No handler for method: {}", method)))
            }
        }
    }
}

/// Forward `raw` (the original request bytes) to the handler component's socket
/// at /run/the-machine/<handler>.sock and return its response bytes.
async fn forward_to(handler_id: &str, raw: &[u8]) -> Option<Vec<u8>> {
    let path = format!("/run/the-machine/{}.sock", handler_id);
    let mut stream = UnixStream::connect(&path).await.ok()?;

    let mut buf = raw.to_vec();
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

fn error_bytes(id: u64, code: &str, message: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "id": id,
        "error": { "code": code, "message": message }
    })).unwrap()
}

/// Map a method to its owning component by the segment before the first dot,
/// so unregistered methods like `ui.get_tree` still reach `ui-runtime`.
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
