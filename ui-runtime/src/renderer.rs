//! Renderer bridge: sync UI tree nodes to compositor surfaces.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::UiTree;

pub fn serialize_subtree(tree: &UiTree, id: &str) -> Value {
    let Some(node) = tree.get(id) else {
        return Value::Null;
    };
    let children: Vec<Value> = node
        .children
        .iter()
        .map(|child_id| serialize_subtree(tree, child_id))
        .filter(|v| !v.is_null())
        .collect();
    json!({
        "id": node.id,
        "type": node.kind,
        "props": node.props,
        "children": children,
    })
}

pub async fn sync_tree_to_compositor(root: &Value) -> usize {
    let mut count = 0;
    if let Some(nodes) = collect_visible_nodes(root) {
        for node in &nodes {
            let id = node
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("ui.unknown");
            let kind = node
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("widget");
            let label = node
                .get("props")
                .and_then(|p| p.get("text").or_else(|| p.get("label")))
                .and_then(|v| v.as_str())
                .unwrap_or(id);
            let (x, y, w, h) = geometry_for(id, kind);
            let _ = bus_call(
                "compositor.surface",
                json!({
                    "action": "create",
                    "id": format!("surface.{}", id),
                    "geometry": { "x": x, "y": y, "width": w, "height": h },
                    "kind": kind,
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

fn geometry_for(id: &str, kind: &str) -> (i32, i32, u32, u32) {
    match id {
        "ui.greeting" => (40, 32, 1200, 64),
        "ui.chat_log" => (40, 110, 1200, 400),
        "ui.chat_input" => (40, 530, 980, 56),
        "ui.chat_send" => (1040, 530, 200, 56),
        _ => match kind {
            "text" => (40, 40, 800, 48),
            "input" => (40, 520, 800, 48),
            "button" => (880, 520, 160, 48),
            _ => (40, 40, 240, 48),
        },
    }
}

fn collect_visible_nodes(root: &Value) -> Option<Vec<Value>> {
    let mut out = Vec::new();
    walk_node(root, &mut out);
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn walk_node(node: &Value, out: &mut Vec<Value>) {
    let kind = node.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if !matches!(kind, "container" | "stack") {
        out.push(node.clone());
    }
    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child in children {
            if child.is_object() {
                walk_node(child, out);
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
    tokio::io::AsyncWriteExt::write_all(&mut stream, &bytes)
        .await
        .ok()?;
    let mut buf = vec![0u8; 65536];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
        .await
        .ok()?;
    let resp: Value = serde_json::from_slice(&buf[..n]).ok()?;
    resp.get("result").cloned()
}
