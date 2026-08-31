//! Tree-owned focus model (tab order + activate + dialog trap).

use crate::UiTree;

/// Interactive kinds that participate in tab order.
pub fn is_interactive(kind: &str) -> bool {
    matches!(
        kind,
        "button"
            | "field"
            | "input"
            | "toggle"
            | "slider"
            | "list"
            | "dialog"
            | "media"
            | "chart"
            | "grid"
    )
}

pub fn focusable_ids(tree: &UiTree) -> Vec<String> {
    let mut out = Vec::new();
    collect(tree.root_id(), tree, &mut out);
    out
}

/// When a dialog is open, Tab cycles only within that dialog subtree (focus trap).
pub fn focusable_ids_trapped(tree: &UiTree) -> Vec<String> {
    if let Some(dialog_id) = find_dialog_id(tree) {
        let mut out = Vec::new();
        collect(&dialog_id, tree, &mut out);
        // Prefer controls inside the dialog; if none, fall back to the dialog itself.
        if out.is_empty() {
            out.push(dialog_id);
        }
        return out;
    }
    focusable_ids(tree)
}

fn find_dialog_id(tree: &UiTree) -> Option<String> {
    find_kind(tree.root_id(), tree, "dialog")
}

fn find_kind(id: &str, tree: &UiTree, kind: &str) -> Option<String> {
    let node = tree.get(id)?;
    if node.kind == kind {
        return Some(node.id.clone());
    }
    for child in &node.children {
        if let Some(found) = find_kind(child, tree, kind) {
            return Some(found);
        }
    }
    None
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
    let ids = focusable_ids_trapped(tree);
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
        assert!(is_interactive("media"));
        assert!(!is_interactive("text"));
        assert!(!is_interactive("stack"));
    }
}
