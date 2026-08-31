//! Renderer bridge: layout AUIL trees with design-system tokens, sync to compositor.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::layout::{self, LaidOutNode};
use crate::tokens;
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
    let (vw, vh) = layout::default_viewport();
    let laid = layout::layout_tree(root, vw, vh);
    let mut count = 0;
    for node in &laid {
        // Skip empty caption/status plates so they don't show widget ids.
        if node.kind == "text" && node.label.is_empty() {
            continue;
        }
        let _ = bus_call(
            "compositor.surface",
            surface_create_params(node),
        )
        .await;
        count += 1;
    }
    let _ = bus_call(
        "compositor.present",
        json!({ "damage": "full", "revision": Uuid::new_v4() }),
    )
    .await;
    count
}

fn surface_create_params(node: &LaidOutNode) -> Value {
    let mut params = json!({
        "action": "create",
        "id": format!("surface.{}", node.id),
        "geometry": {
            "x": node.x,
            "y": node.y,
            "width": node.width,
            "height": node.height,
        },
        "kind": node.kind,
        "label": node.label,
        "variant": node.variant,
        "radius": node.radius,
        "font_scale": node.font_scale,
        "bg": node.bg.to_array(),
        "fg": node.fg.to_array(),
    });
    if let Some(border) = node.border {
        params
            .as_object_mut()
            .unwrap()
            .insert("border".into(), json!(border.to_array()));
    }
    // Focus ring for the primary input.
    if node.id == "ui.chat_input" {
        params.as_object_mut().unwrap().insert(
            "border".into(),
            json!(tokens::dark::BORDER_FOCUS.to_array()),
        );
    }
    params
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_params_carry_design_tokens() {
        let node = LaidOutNode {
            id: "ui.chat_send".into(),
            kind: "button".into(),
            label: "Send".into(),
            placeholder: String::new(),
            x: 10,
            y: 20,
            width: 96,
            height: 48,
            bg: tokens::dark::ACCENT_DEFAULT,
            fg: tokens::dark::TEXT_ON_ACCENT,
            border: None,
            radius: tokens::radius::MD,
            font_scale: 3,
            variant: "primary".into(),
            role: String::new(),
        };
        let p = surface_create_params(&node);
        assert_eq!(p.get("kind").and_then(|v| v.as_str()), Some("button"));
        assert_eq!(p.get("variant").and_then(|v| v.as_str()), Some("primary"));
        assert_eq!(
            p.get("bg").and_then(|v| v.as_array()).map(|a| a.len()),
            Some(3)
        );
        assert_eq!(p.get("radius").and_then(|v| v.as_u64()), Some(10));
    }
}
