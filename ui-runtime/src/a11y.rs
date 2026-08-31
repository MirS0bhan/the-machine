//! Accessibility tree export (AT-SPI-shaped MCP surface, P2).

use serde_json::{json, Value};

use crate::UiTree;

/// Map AUIL kinds to ARIA / AT-SPI-ish roles.
pub fn role_for(kind: &str, props: &std::collections::HashMap<String, Value>) -> &'static str {
    if let Some(r) = props.get("role").and_then(|v| v.as_str()) {
        return match r {
            "title" => "heading",
            "caption" => "text",
            _ => "generic",
        };
    }
    match kind {
        "button" => "button",
        "field" | "input" => "textfield",
        "toggle" => "switch",
        "slider" => "slider",
        "list" => "list",
        "dialog" => "dialog",
        "text" => "label",
        "icon" => "image",
        "media" => "multimedia",
        "chart" => "image",
        "stack" | "container" | "grid" => "panel",
        _ => "generic",
    }
}

pub fn name_for(kind: &str, id: &str, props: &std::collections::HashMap<String, Value>) -> String {
    props
        .get("label")
        .or_else(|| props.get("text"))
        .or_else(|| props.get("aria-label"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            if kind == "field" || kind == "input" {
                props
                    .get("placeholder")
                    .and_then(|v| v.as_str())
                    .unwrap_or(id)
                    .to_string()
            } else {
                id.to_string()
            }
        })
}

pub fn serialize_tree(tree: &UiTree) -> Value {
    serialize_node(tree, tree.root_id())
}

fn serialize_node(tree: &UiTree, id: &str) -> Value {
    let Some(node) = tree.get(id) else {
        return Value::Null;
    };
    let role = role_for(&node.kind, &node.props);
    let name = name_for(&node.kind, &node.id, &node.props);
    let focused = tree.focused() == Some(node.id.as_str());
    let disabled = node
        .props
        .get("disabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let checked = node
        .props
        .get("checked")
        .and_then(|v| v.as_bool());
    let value = node.props.get("value").cloned();
    let children: Vec<Value> = node
        .children
        .iter()
        .map(|c| serialize_node(tree, c))
        .filter(|v| !v.is_null())
        .collect();
    json!({
        "id": node.id,
        "role": role,
        "name": name,
        "kind": node.kind,
        "states": {
            "focused": focused,
            "disabled": disabled,
            "checked": checked,
        },
        "value": value,
        "live": node.props.get("live").cloned().unwrap_or(Value::Null),
        "children": children,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn button_role() {
        assert_eq!(role_for("button", &HashMap::new()), "button");
        assert_eq!(role_for("field", &HashMap::new()), "textfield");
    }
}
