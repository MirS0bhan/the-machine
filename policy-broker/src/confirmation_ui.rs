//! Confirmation Surface Daemon — renders broker CONFIRM/HOLD prompts (policy-broker-spec §9).
//!
//! Owns the confirmation UI exclusively; agent-core must not render approve/deny controls.

use serde_json::json;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use uuid::Uuid;

use crate::confirmation::ConfirmationDaemon;

pub async fn run_confirmation_ui_loop(confirmation: Arc<tokio::sync::Mutex<ConfirmationDaemon>>) {
    let mut last_rendered: Option<String> = None;
    loop {
        let pending = {
            let c = confirmation.lock().await;
            c.list_pending()
        };
        if pending.is_empty() {
            if last_rendered.is_some() {
                let _ = deactivate_confirmation_surface().await;
                last_rendered = None;
            }
        } else {
            let first = &pending[0];
            if last_rendered.as_deref() != Some(&first.correlation_id)
                && render_confirmation(
                    &first.correlation_id,
                    &first.capability,
                    first.path.as_deref(),
                )
                .await
            {
                last_rendered = Some(first.correlation_id.clone());
                info!("confirmation surface shown for {}", first.correlation_id);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

async fn render_confirmation(correlation_id: &str, capability: &str, path: Option<&str>) -> bool {
    let surface_id = "surface.confirmation";
    let widget_approve = "ui.confirmation.approve";
    let widget_deny = "ui.confirmation.deny";
    let body = path.unwrap_or("(no path)");

    let _ = bus_call(
        "compositor.surface",
        json!({
            "action": "create",
            "id": surface_id,
            "geometry": { "x": 200, "y": 150, "width": 880, "height": 420 },
            "kind": "confirmation",
            "label": format!("Confirm: {}", capability),
            "confirmation": true,
        }),
    )
    .await;

    let _ = bus_call(
        "compositor.confirmation.set_active",
        json!({ "active": true, "surface_id": surface_id }),
    )
    .await;

    let patch = json!({
        "ops": [
            {
                "op": "replace",
                "id": "ui.confirmation.root",
                "node": {
                    "id": "ui.confirmation.root",
                    "type": "container",
                    "props": { "title": "Policy Confirmation Required" },
                    "children": [widget_approve, widget_deny, "ui.confirmation.body"]
                }
            },
            {
                "op": "insert",
                "anchor": "ui.confirmation.root",
                "node": {
                    "id": "ui.confirmation.body",
                    "type": "text",
                    "props": {
                        "text": format!("Capability: {}\nPath: {}\nCorrelation: {}", capability, body, correlation_id)
                    }
                }
            },
            {
                "op": "insert",
                "anchor": "ui.confirmation.root",
                "node": {
                    "id": widget_approve,
                    "type": "button",
                    "props": { "label": "Approve", "correlation_id": correlation_id, "approved": true },
                    "bindings": [{
                        "type": "mcp",
                        "target": "policy.confirm"
                    }]
                }
            },
            {
                "op": "insert",
                "anchor": "ui.confirmation.root",
                "node": {
                    "id": widget_deny,
                    "type": "button",
                    "props": { "label": "Deny", "correlation_id": correlation_id, "approved": false },
                    "bindings": [{
                        "type": "mcp",
                        "target": "policy.confirm"
                    }]
                }
            }
        ]
    });

    bus_call("ui.patch", patch).await.is_some()
}

async fn deactivate_confirmation_surface() -> bool {
    let _ = bus_call(
        "compositor.confirmation.set_active",
        json!({ "active": false }),
    )
    .await;
    bus_call(
        "compositor.surface",
        json!({ "action": "destroy", "id": "surface.confirmation" }),
    )
    .await
    .is_some()
}

async fn bus_call(method: &str, params: serde_json::Value) -> Option<serde_json::Value> {
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
    if stream.write_all(&bytes).await.is_err() {
        warn!("confirmation ui: bus write failed for {}", method);
        return None;
    }
    let mut buf = vec![0u8; 65536];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
        .await
        .ok()?;
    if n == 0 {
        return None;
    }
    let resp: serde_json::Value = serde_json::from_slice(&buf[..n]).ok()?;
    resp.get("result").cloned()
}
