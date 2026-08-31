//! Renderer bridge: layout AUIL trees with design-system tokens, sync to compositor.

use serde_json::{json, Value};
use std::collections::HashSet;
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
    let mut desired: HashSet<String> = HashSet::new();
    let mut count = 0;
    for node in &laid {
        // Skip empty caption/status plates so they don't show widget ids.
        if node.kind == "text" && node.label.is_empty() {
            continue;
        }
        desired.insert(format!("surface.{}", node.id));
        ensure_surface(node).await;
        count += 1;
    }
    destroy_orphan_surfaces(&desired).await;
    let _ = bus_call(
        "compositor.present",
        json!({ "damage": "full", "revision": Uuid::new_v4() }),
    )
    .await;
    count
}

/// Destroy compositor surfaces created for UI nodes that are no longer in the tree.
/// Skips confirmation / non-`surface.*` ids.
async fn destroy_orphan_surfaces(desired: &HashSet<String>) {
    let Some(list) = bus_call("compositor.list", json!({})).await else {
        return;
    };
    let surfaces = match list.as_array() {
        Some(a) => a,
        None => return,
    };
    for s in surfaces {
        let Some(id) = s.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        if !id.starts_with("surface.") {
            continue;
        }
        if s.get("confirmation")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }
        if desired.contains(id) {
            continue;
        }
        let _ = bus_call(
            "compositor.surface",
            json!({ "action": "destroy", "id": id }),
        )
        .await;
    }
}

fn surface_create_params(node: &LaidOutNode) -> Value {
    let font_px = match node.font_scale {
        4 => 20,
        3 => 13,
        2 => 14,
        1 => 12,
        _ => 14,
    };
    let font_weight = if node.role == "title" || node.id == "ui.greeting" {
        "bold"
    } else if node.kind == "button" {
        "medium"
    } else {
        "regular"
    };
    let mut params = json!({
        "action": "update",
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
        "font_px": font_px,
        "font_weight": font_weight,
        "font_family": "default",
        "bg": node.bg.to_array(),
        "fg": node.fg.to_array(),
        "pressed": node.pressed,
        "checked": node.checked,
        "value": node.value,
        "value_min": node.value_min,
        "value_max": node.value_max,
        "scroll_y": node.scroll_y,
        "items": node.items,
    });
    if let Some(border) = node.border {
        params
            .as_object_mut()
            .unwrap()
            .insert("border".into(), json!(border.to_array()));
    }
    if node.id == "ui.chat_input" {
        params.as_object_mut().unwrap().insert(
            "border".into(),
            json!(tokens::dark::BORDER_FOCUS.to_array()),
        );
    }
    // Dialog cards sit above normal chrome.
    if node.kind == "dialog" {
        params
            .as_object_mut()
            .unwrap()
            .insert("z_order".into(), json!(5_000));
    }
    params
}

async fn ensure_surface(node: &LaidOutNode) {
    let mut create = surface_create_params(node);
    create
        .as_object_mut()
        .unwrap()
        .insert("action".into(), json!("create"));
    let updated = bus_call("compositor.surface", surface_create_params(node)).await;
    let ok = updated
        .as_ref()
        .and_then(|v| v.get("ok"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !ok {
        let _ = bus_call("compositor.surface", create).await;
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
            pressed: false,
            checked: false,
            value: 0.0,
            value_min: 0.0,
            value_max: 100.0,
            scroll_y: 0,
            items: vec![],
        };
        let p = surface_create_params(&node);
        assert_eq!(p.get("kind").and_then(|v| v.as_str()), Some("button"));
        assert_eq!(p.get("variant").and_then(|v| v.as_str()), Some("primary"));
        assert_eq!(
            p.get("bg").and_then(|v| v.as_array()).map(|a| a.len()),
            Some(3)
        );
        assert_eq!(p.get("radius").and_then(|v| v.as_u64()), Some(10));
        assert_eq!(p.get("pressed").and_then(|v| v.as_bool()), Some(false));
    }
}
