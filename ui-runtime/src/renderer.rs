//! Renderer bridge: sync UI tree nodes to compositor surfaces.

use serde_json::{json, Value};
use uuid::Uuid;

pub async fn sync_tree_to_compositor(root: &Value) -> usize {
    let mut count = 0;
    if let Some(nodes) = collect_visible_nodes(root) {
        for (idx, node) in nodes.iter().enumerate() {
            let id = node
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("ui.unknown");
            let label = node
                .get("props")
                .and_then(|p| p.get("label"))
                .and_then(|v| v.as_str())
                .unwrap_or(id);
            let x = 20 + (idx as i32 % 4) * 220;
            let y = 20 + (idx as i32 / 4) * 80;
            let _ = bus_call(
                "compositor.surface",
                json!({
                    "action": "create",
                    "id": format!("surface.{}", id),
                    "geometry": { "x": x, "y": y, "width": 200, "height": 60 },
                    "kind": "widget",
                    "label": label,
                }),
            )
            .await;
            count += 1;
        }
    }
    let _ = bus_call(
        "compositor.present",
        json!({ "damage": "full", "revision": Uuid::new_v4() }),
    )
    .await;
    count
}

fn collect_visible_nodes(root: &Value) -> Option<Vec<Value>> {
    let mut out = Vec::new();
    walk_node(root, &mut out);
    if out.is_empty() { None } else { Some(out) }
}

fn walk_node(node: &Value, out: &mut Vec<Value>) {
    if node.get("type").and_then(|v| v.as_str()) != Some("container") {
        out.push(node.clone());
    }
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child_id in children {
            if let Some(cid) = child_id.as_str() {
                if cid.starts_with("ui.") {
                    out.push(json!({ "id": cid, "type": "widget", "props": { "label": cid } }));
                }
            }
        }
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
    tokio::io::AsyncWriteExt::write_all(&mut stream, &bytes).await.ok()?;
    let mut buf = vec![0u8; 65536];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await.ok()?;
    let resp: Value = serde_json::from_slice(&buf[..n]).ok()?;
    resp.get("result").cloned()
}
