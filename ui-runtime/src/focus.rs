//! Tree-owned focus model (tab order + activate).

use crate::UiTree;

/// Interactive kinds that participate in tab order.
pub fn is_interactive(kind: &str) -> bool {
    matches!(
        kind,
        "button" | "field" | "input" | "toggle" | "slider" | "list" | "dialog"
    )
}

pub fn focusable_ids(tree: &UiTree) -> Vec<String> {
    let mut out = Vec::new();
    collect(tree.root_id(), tree, &mut out);
    out
}

fn collect(id: &str, tree: &UiTree, out: &mut Vec<String>) {
    if let Some(node) = tree.get(id) {
        if is_interactive(&node.kind) {
            let disabled = node
                .props
                .get("disabled")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !disabled {
                out.push(id.to_string());
            }
        }
        for child in &node.children {
            collect(child, tree, out);
        }
    }
}

pub fn next_focus(tree: &UiTree, current: Option<&str>, reverse: bool) -> Option<String> {
    let ids = focusable_ids(tree);
    if ids.is_empty() {
        return None;
    }
    let idx = current
        .and_then(|c| ids.iter().position(|id| id == c))
        .unwrap_or(if reverse { 0 } else { ids.len() - 1 });
    if reverse {
        Some(ids[(idx + ids.len() - 1) % ids.len()].clone())
    } else {
        Some(ids[(idx + 1) % ids.len()].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_kinds() {
        assert!(is_interactive("field"));
        assert!(is_interactive("button"));
        assert!(is_interactive("toggle"));
        assert!(!is_interactive("text"));
        assert!(!is_interactive("stack"));
    }
}
