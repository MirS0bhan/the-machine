//! Tree-owned focus model (tab order + activate + dialog trap).

use crate::UiTree;

/// Interactive kinds that participate in tab order.
pub fn is_interactive(kind: &str) -> bool {
    matches!(
        kind,
        "button" | "field" | "input" | "toggle" | "slider" | "list" | "dialog" | "media"
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

/// Top-level surfaces for Alt+Tab: agent-placed workspace controls, any open
/// dialog or menu, then the chat field as the always-present home surface.
///
/// This is "surface" in the compositor sense — each of these owns a
/// `surface.<id>` — not every focusable leaf, so Alt+Tab steps between
/// app-sized things rather than walking the whole tab ring.
pub fn top_level_surfaces(tree: &UiTree) -> Vec<String> {
    let mut out = Vec::new();
    // Dialogs and menus are modal-ish and come first.
    collect_surfaces(tree.root_id(), tree, &mut out, &|node| {
        node.kind == "dialog" || node.props.get("surface").and_then(|v| v.as_str()) == Some("menu")
    });
    if let Some(workspace) = tree.get("ui.workspace") {
        for child in &workspace.children {
            if let Some(node) = tree.get(child) {
                if is_interactive(&node.kind) && !out.contains(&node.id) {
                    out.push(node.id.clone());
                }
            }
        }
    }
    // The shell chrome is one surface, not one entry per control, so Alt+Tab
    // steps between app-sized things instead of walking the chat row twice.
    for home in ["ui.chat_input", "ui.chat_send", "ui.root"] {
        if tree.get(home).is_some() {
            if !out.contains(&home.to_string()) {
                out.push(home.to_string());
            }
            break;
        }
    }
    out
}

fn collect_surfaces(
    id: &str,
    tree: &UiTree,
    out: &mut Vec<String>,
    pred: &dyn Fn(&crate::UiNode) -> bool,
) {
    if let Some(node) = tree.get(id) {
        if pred(node) && !out.contains(&node.id) {
            out.push(node.id.clone());
        }
        for child in &node.children {
            collect_surfaces(child, tree, out, pred);
        }
    }
}

/// Ordered focusable ids with their roles, for `ui.a11y.focus_order`.
pub fn focus_order(tree: &UiTree) -> Vec<(String, &'static str, String)> {
    focusable_ids_trapped(tree)
        .into_iter()
        .filter_map(|id| {
            let node = tree.get(&id)?;
            Some((
                id.clone(),
                crate::a11y::role_for(&node.kind, &node.props),
                crate::a11y::name_for(&node.kind, &node.id, &node.props),
            ))
        })
        .collect()
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
