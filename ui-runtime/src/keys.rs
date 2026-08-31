//! Keyboard event handling for the boot shell.
//!
//! A key event resolves against the shortcut table first (so a user-installed
//! chord wins), then falls through to text editing on the focused field. Every
//! branch returns a described outcome so callers — and the scenario suite — can
//! assert what a key actually did.

use serde_json::{json, Value};

use crate::{
    a11y, focus, ime, input_edit, mcp_call, remove_subtree, renderer, scroll, shortcuts,
    SharedTree, UiNode,
};

/// What a key press resolved to.
pub enum KeyOutcome {
    /// Fully handled; the value is the MCP result body.
    Handled(Value),
    /// Run the bindings of this node as a press (Enter on a focused button).
    Activate(String),
    /// Not a shell concern — let the caller run node bindings.
    Pass,
}

#[derive(Clone, Debug)]
pub struct KeyEvent {
    pub key: String,
    pub text: Option<String>,
    pub mods: input_edit::KeyMods,
}

impl KeyEvent {
    pub fn from_payload(payload: &Value) -> Self {
        KeyEvent {
            key: payload
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            text: payload
                .get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            mods: input_edit::KeyMods {
                shift: flag(payload, "shift"),
                ctrl: flag(payload, "ctrl"),
                alt: flag(payload, "alt"),
                meta: flag(payload, "meta"),
            },
        }
    }

    pub fn chord(&self) -> String {
        shortcuts::chord_for(
            &self.key,
            self.mods.ctrl,
            self.mods.alt,
            self.mods.shift,
            self.mods.meta,
        )
    }
}

fn flag(payload: &Value, name: &str) -> bool {
    payload.get(name).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Viewport height used for list scrolling when the node does not declare one.
fn list_viewport(node: &UiNode) -> u32 {
    node.props
        .get("height")
        .and_then(|v| v.as_u64())
        .unwrap_or(160) as u32
}

pub async fn handle(tree: &SharedTree, nid: &str, payload: &Value) -> KeyOutcome {
    let ev = KeyEvent::from_payload(payload);
    if ev.key.is_empty() {
        return KeyOutcome::Pass;
    }

    // A pending IME sequence owns Escape before any shortcut sees it.
    if ev.key == "Escape" {
        let cancelled = {
            let mut t = tree.lock().await;
            if t.ime.pending.is_some() {
                t.ime.reset();
                true
            } else {
                false
            }
        };
        if cancelled {
            return KeyOutcome::Handled(json!({ "handled": 1, "action": "ime.cancel" }));
        }
    }

    if let Some(sc) = shortcuts::resolve(&ev.chord()) {
        match run_action(tree, nid, &sc, &ev).await {
            KeyOutcome::Pass => {}
            other => return other,
        }
    }

    // Compose / dead-key IME before ordinary insertion.
    if !(ev.mods.ctrl || ev.mods.meta) {
        match feed_ime(tree, nid, &ev).await {
            KeyOutcome::Pass => {}
            other => return other,
        }
    }

    edit_focused_field(tree, nid, &ev).await
}

async fn run_action(
    tree: &SharedTree,
    nid: &str,
    sc: &shortcuts::Shortcut,
    ev: &KeyEvent,
) -> KeyOutcome {
    match sc.action.as_str() {
        "focus.next" | "focus.previous" => {
            let reverse = sc.action == "focus.previous" || ev.mods.shift;
            let next = {
                let mut t = tree.lock().await;
                let next = focus::next_focus(&t, t.focused(), reverse);
                t.set_focused(next.clone());
                next
            };
            if let Some(sid) = &next {
                let _ = mcp_call(
                    "compositor.focus",
                    json!({ "id": format!("surface.{sid}") }),
                )
                .await;
            }
            KeyOutcome::Handled(json!({
                "handled": 1,
                "action": "tab",
                "focused": next,
                "reverse": reverse,
            }))
        }
        "surface.cycle" | "surface.cycle.reverse" => {
            let reverse = sc.action.ends_with("reverse") || ev.mods.shift;
            cycle_surface(tree, reverse).await
        }
        "dismiss" => dismiss(tree).await,
        "activate" => {
            let focus_id = {
                let t = tree.lock().await;
                t.focused().unwrap_or(nid).to_string()
            };
            let kind = {
                let t = tree.lock().await;
                t.get(&focus_id).map(|n| n.kind.clone()).unwrap_or_default()
            };
            match kind.as_str() {
                "button" | "toggle" | "list" | "media" => KeyOutcome::Activate(focus_id),
                // Enter in a text field submits the field's own bindings.
                "field" | "input" => KeyOutcome::Activate(focus_id),
                _ => KeyOutcome::Pass,
            }
        }
        "select.all" => {
            let changed = mutate_field(tree, nid, |buf| {
                buf.select_all();
                true
            })
            .await;
            match changed {
                Some(state) => {
                    sync(tree).await;
                    KeyOutcome::Handled(json!({
                        "handled": 1,
                        "action": "select.all",
                        "selection": state.selection,
                    }))
                }
                None => KeyOutcome::Pass,
            }
        }
        "undo" | "redo" => {
            let redo = sc.action == "redo";
            let changed =
                mutate_field(tree, nid, |buf| if redo { buf.redo() } else { buf.undo() }).await;
            match changed {
                Some(state) => {
                    sync(tree).await;
                    KeyOutcome::Handled(json!({
                        "handled": 1,
                        "action": sc.action,
                        "text": state.text,
                        "caret": state.caret,
                    }))
                }
                None => KeyOutcome::Handled(json!({
                    "handled": 0,
                    "action": sc.action,
                    "reason": "nothing to undo",
                })),
            }
        }
        "clipboard.copy" | "clipboard.cut" => {
            clipboard_copy(tree, nid, sc.action == "clipboard.cut").await
        }
        "clipboard.paste" => clipboard_paste(tree, nid).await,
        "scroll.page_up" | "scroll.page_down" => {
            let pages = if sc.action == "scroll.page_up" { -1 } else { 1 };
            scroll_focused(tree, nid, pages).await
        }
        "menu.open" => open_menu(tree).await,
        "snapshot" => snapshot(tree).await,
        "workspace.clear" => {
            // Shortcuts that mutate other components go over the bus so policy
            // still applies; nothing is bypassed by binding a chord to it.
            let method = sc
                .method
                .clone()
                .unwrap_or_else(|| "ui.workspace.clear".into());
            let result = mcp_call(&method, json!({ "anchor": "ui.workspace" })).await;
            KeyOutcome::Handled(json!({
                "handled": u8::from(result.is_some()),
                "action": "workspace.clear",
                "result": result,
            }))
        }
        "a11y.tree" => {
            let tree_json = {
                let t = tree.lock().await;
                a11y::serialize_tree(&t)
            };
            KeyOutcome::Handled(json!({
                "handled": 1,
                "action": "a11y.tree",
                "tree": tree_json,
            }))
        }
        _ => KeyOutcome::Pass,
    }
}

/// Focus the next top-level surface (workspace control, then chrome).
async fn cycle_surface(tree: &SharedTree, reverse: bool) -> KeyOutcome {
    let next = {
        let mut t = tree.lock().await;
        let surfaces = focus::top_level_surfaces(&t);
        if surfaces.is_empty() {
            None
        } else {
            let current = t.focused().map(|s| s.to_string());
            let idx = current
                .as_ref()
                .and_then(|c| surfaces.iter().position(|s| s == c))
                .unwrap_or(if reverse { 0 } else { surfaces.len() - 1 });
            let next = if reverse {
                surfaces[(idx + surfaces.len() - 1) % surfaces.len()].clone()
            } else {
                surfaces[(idx + 1) % surfaces.len()].clone()
            };
            t.set_focused(Some(next.clone()));
            Some(next)
        }
    };
    if let Some(sid) = &next {
        let _ = mcp_call(
            "compositor.focus",
            json!({ "id": format!("surface.{sid}") }),
        )
        .await;
    }
    KeyOutcome::Handled(json!({
        "handled": u8::from(next.is_some()),
        "action": "surface.cycle",
        "focused": next,
        "reverse": reverse,
    }))
}

/// Escape: close the open menu, else dismiss the top dialog.
async fn dismiss(tree: &SharedTree) -> KeyOutcome {
    let removed = {
        let mut t = tree.lock().await;
        let menu = t
            .nodes
            .values()
            .find(|n| n.props.get("surface").and_then(|v| v.as_str()) == Some("menu"))
            .map(|n| n.id.clone());
        let target = menu.or_else(|| {
            t.nodes
                .values()
                .find(|n| n.kind == "dialog")
                .map(|n| n.id.clone())
        });
        match target {
            Some(did) => {
                t.detach(&did);
                remove_subtree(&mut t, &did);
                if t.focused()
                    .map(|f| !t.nodes.contains_key(f))
                    .unwrap_or(false)
                {
                    t.set_focused(None);
                }
                t.revision += 1;
                Some(did)
            }
            None => None,
        }
    };
    let Some(did) = removed else {
        return KeyOutcome::Pass;
    };
    sync(tree).await;
    let _ = mcp_call(
        "compositor.surface",
        json!({ "action": "destroy", "id": format!("surface.{did}") }),
    )
    .await;
    KeyOutcome::Handled(json!({
        "handled": 1,
        "action": "dialog.dismiss",
        "id": did,
    }))
}

async fn clipboard_copy(tree: &SharedTree, nid: &str, cut: bool) -> KeyOutcome {
    let (payload, kind) = {
        let t = tree.lock().await;
        let focus_id = t.focused().unwrap_or(nid).to_string();
        match t.get(&focus_id) {
            Some(node) if matches!(node.kind.as_str(), "field" | "input") => {
                let buf = buffer_from(node);
                let selected = buf.selected_text();
                let text = if selected.is_empty() {
                    buf.text.clone()
                } else {
                    selected
                };
                (Some((focus_id, text)), "field")
            }
            // Copying from a list yields the highlighted row, which is what a
            // user pressing Ctrl+C over a list expects.
            Some(node) if node.kind == "list" => {
                let idx = node
                    .props
                    .get("selected")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                let row = node
                    .props
                    .get("items")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.get(idx))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                (Some((focus_id, row)), "list")
            }
            Some(node) if node.kind == "text" => {
                let text = node
                    .props
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                (Some((focus_id, text)), "text")
            }
            _ => (None, ""),
        }
    };
    let Some((focus_id, text)) = payload else {
        return KeyOutcome::Pass;
    };
    if text.is_empty() {
        return KeyOutcome::Handled(json!({
            "handled": 0,
            "action": if cut { "clipboard.cut" } else { "clipboard.copy" },
            "reason": "nothing selected",
        }));
    }
    let stored = mcp_call("clipboard.set", json!({ "text": text.clone() })).await;
    if cut && kind == "field" {
        let changed = mutate_field_id(tree, &focus_id, |buf| {
            if buf.selection().is_some() {
                buf.delete_selection()
            } else {
                buf.select_all();
                buf.delete_selection()
            }
        })
        .await;
        if changed.is_some() {
            sync(tree).await;
        }
    }
    KeyOutcome::Handled(json!({
        "handled": u8::from(stored.is_some()),
        "action": if cut { "clipboard.cut" } else { "clipboard.copy" },
        "text": text,
        "source": kind,
    }))
}

async fn clipboard_paste(tree: &SharedTree, nid: &str) -> KeyOutcome {
    let focus_id = {
        let t = tree.lock().await;
        t.focused().unwrap_or(nid).to_string()
    };
    let editable = {
        let t = tree.lock().await;
        t.get(&focus_id)
            .is_some_and(|n| matches!(n.kind.as_str(), "field" | "input"))
    };
    if !editable {
        return KeyOutcome::Handled(json!({
            "handled": 0,
            "action": "clipboard.paste",
            "reason": "focused node is not editable",
        }));
    }
    let clip = mcp_call("clipboard.get", json!({})).await;
    let Some(text) = clip
        .as_ref()
        .and_then(|v| v.get("text"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
    else {
        // Fail soft and say so, rather than pretending a paste happened.
        return KeyOutcome::Handled(json!({
            "handled": 0,
            "action": "clipboard.paste",
            "reason": "clipboard unavailable",
        }));
    };
    if text.is_empty() {
        return KeyOutcome::Handled(json!({
            "handled": 0,
            "action": "clipboard.paste",
            "reason": "clipboard empty",
        }));
    }
    let state = mutate_field_id(tree, &focus_id, |buf| {
        buf.insert_str(&text);
        true
    })
    .await;
    sync(tree).await;
    KeyOutcome::Handled(json!({
        "handled": 1,
        "action": "clipboard.paste",
        "text": state.map(|s| s.text),
    }))
}

async fn scroll_focused(tree: &SharedTree, nid: &str, pages: i32) -> KeyOutcome {
    let target = {
        let t = tree.lock().await;
        let focus_id = t.focused().unwrap_or(nid).to_string();
        match t.get(&focus_id) {
            Some(node) if node.kind == "list" => Some(focus_id),
            _ => None,
        }
    };
    let Some(target) = target else {
        return KeyOutcome::Pass;
    };
    let offset = {
        let mut t = tree.lock().await;
        let vh = t.get(&target).map(list_viewport).unwrap_or(160);
        let node = t.get_mut(&target).expect("target exists");
        scroll::apply_page(&mut node.props, pages, vh);
        let offset = node.props.get("scroll_y").cloned();
        t.revision += 1;
        offset
    };
    sync(tree).await;
    KeyOutcome::Handled(json!({
        "handled": 1,
        "action": if pages < 0 { "scroll.page_up" } else { "scroll.page_down" },
        "id": target,
        "scroll_y": offset,
    }))
}

/// Command menu composed from the `list` primitive (no thirteenth kind).
pub const MENU_ID: &str = "ui.command_menu";

pub fn menu_node() -> Value {
    json!({
        "id": MENU_ID,
        "type": "list",
        "props": {
            "label": "Commands",
            "surface": "menu",
            "items": [
                "Show agent status",
                "Clear the workspace",
                "Read the accessibility tree",
                "List keyboard shortcuts",
                "Give me a tour",
            ],
            "selected": 0,
            "scroll_y": 0,
            "live": "polite",
        },
        "bindings": [{ "type": "mcp", "target": "agent.chat.send", "event": "activate" }],
        "children": [],
    })
}

async fn open_menu(tree: &SharedTree) -> KeyOutcome {
    let ops = vec![json!({
        "op": "insert",
        "anchor": "ui.root",
        "position": "child",
        "node": menu_node(),
    })];
    let applied = crate::apply_patch(tree, ops, true).await;
    match applied {
        Ok(rev) => {
            {
                let mut t = tree.lock().await;
                t.set_focused(Some(MENU_ID.to_string()));
            }
            let _ = mcp_call(
                "compositor.focus",
                json!({ "id": format!("surface.{MENU_ID}") }),
            )
            .await;
            KeyOutcome::Handled(json!({
                "handled": 1,
                "action": "menu.open",
                "id": MENU_ID,
                "revision": rev,
            }))
        }
        Err(e) => KeyOutcome::Handled(json!({
            "handled": 0,
            "action": "menu.open",
            "error": e,
        })),
    }
}

async fn snapshot(tree: &SharedTree) -> KeyOutcome {
    let snap = {
        let t = tree.lock().await;
        crate::snapshot_value(&t)
    };
    KeyOutcome::Handled(json!({
        "handled": 1,
        "action": "snapshot",
        "snapshot": snap,
    }))
}

async fn feed_ime(tree: &SharedTree, nid: &str, ev: &KeyEvent) -> KeyOutcome {
    let out = {
        let mut t = tree.lock().await;
        t.ime.feed(&ev.key, ev.text.as_deref())
    };
    match out {
        ime::ImeOutput::Pending => {
            let preedit = {
                let t = tree.lock().await;
                t.ime.pending.clone().unwrap_or_default()
            };
            // Show the dead key so the user knows the shell is composing.
            let focus_id = {
                let t = tree.lock().await;
                t.focused().unwrap_or(nid).to_string()
            };
            {
                let mut t = tree.lock().await;
                if let Some(node) = t.get_mut(&focus_id) {
                    node.props.insert("preedit".into(), json!(preedit.clone()));
                }
                t.revision += 1;
            }
            sync(tree).await;
            KeyOutcome::Handled(json!({
                "handled": 1,
                "action": "ime.pending",
                "preedit": preedit,
            }))
        }
        ime::ImeOutput::Commit(composed) => {
            let state = mutate_field(tree, nid, |buf| {
                buf.insert_str(&composed);
                true
            })
            .await;
            if state.is_none() {
                return KeyOutcome::Pass;
            }
            {
                let mut t = tree.lock().await;
                let focus_id = t.focused().unwrap_or(nid).to_string();
                if let Some(node) = t.get_mut(&focus_id) {
                    node.props.remove("preedit");
                }
            }
            sync(tree).await;
            KeyOutcome::Handled(json!({
                "handled": 1,
                "action": "ime.commit",
                "text": composed,
            }))
        }
        ime::ImeOutput::Pass => KeyOutcome::Pass,
    }
}

async fn edit_focused_field(tree: &SharedTree, nid: &str, ev: &KeyEvent) -> KeyOutcome {
    // Arrow / Home / End / PageUp / PageDown over a focused list navigate rows.
    let list_target = {
        let t = tree.lock().await;
        let focus_id = t.focused().unwrap_or(nid).to_string();
        match t.get(&focus_id) {
            Some(node) if node.kind == "list" => Some(focus_id),
            _ => None,
        }
    };
    if let Some(target) = list_target {
        if let Some(delta) = list_nav_delta(&ev.key) {
            let (selected, offset) = {
                let mut t = tree.lock().await;
                let vh = t.get(&target).map(list_viewport).unwrap_or(160);
                let node = t.get_mut(&target).expect("target exists");
                let selected = match delta {
                    ListNav::By(d) => scroll::move_selection(&mut node.props, d, vh),
                    ListNav::First => scroll::set_selection(&mut node.props, 0, vh),
                    ListNav::Last => scroll::set_selection(&mut node.props, usize::MAX, vh),
                };
                let offset = node.props.get("scroll_y").cloned();
                t.revision += 1;
                (selected, offset)
            };
            sync(tree).await;
            return KeyOutcome::Handled(json!({
                "handled": 1,
                "action": "list.navigate",
                "id": target,
                "selected": selected,
                "scroll_y": offset,
            }));
        }
    }

    let outcome = {
        let mut t = tree.lock().await;
        let focus_id = t.focused().unwrap_or(nid).to_string();
        let Some(node) = t.get(&focus_id) else {
            return KeyOutcome::Pass;
        };
        if !matches!(node.kind.as_str(), "field" | "input") {
            return KeyOutcome::Pass;
        }
        let mut buf = buffer_from(node);
        let outcome = input_edit::apply_key_ext(&mut buf, &ev.key, ev.text.as_deref(), &ev.mods);
        if outcome == input_edit::EditOutcome::Unhandled {
            None
        } else {
            let state = FieldState::from(&buf);
            write_buffer(&mut t, &focus_id, &buf);
            t.revision += 1;
            Some((outcome, state, t.revision))
        }
    };
    let Some((outcome, state, revision)) = outcome else {
        return KeyOutcome::Pass;
    };
    sync(tree).await;
    KeyOutcome::Handled(json!({
        "handled": 1,
        "action": if outcome == input_edit::EditOutcome::Moved { "caret" } else { "edit" },
        "text": state.text,
        "caret": state.caret,
        "selection": state.selection,
        "revision": revision,
    }))
}

enum ListNav {
    By(i32),
    First,
    Last,
}

fn list_nav_delta(key: &str) -> Option<ListNav> {
    match key {
        "ArrowDown" | "Down" => Some(ListNav::By(1)),
        "ArrowUp" | "Up" => Some(ListNav::By(-1)),
        "Home" => Some(ListNav::First),
        "End" => Some(ListNav::Last),
        _ => None,
    }
}

/// Snapshot of a field after an edit, for the MCP response.
pub struct FieldState {
    pub text: String,
    pub caret: usize,
    pub selection: Option<[usize; 2]>,
}

impl FieldState {
    fn from(buf: &input_edit::TextBuffer) -> Self {
        FieldState {
            text: buf.text.clone(),
            caret: buf.caret,
            selection: buf.selection().map(|(a, b)| [a, b]),
        }
    }
}

pub(crate) fn buffer_from(node: &UiNode) -> input_edit::TextBuffer {
    let text = node
        .props
        .get("text")
        .or_else(|| node.props.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let caret = node
        .props
        .get("caret")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);
    let anchor = node
        .props
        .get("selection_anchor")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);
    let mut buf = input_edit::TextBuffer::from_props_with_selection(text, caret, anchor);
    buf.seed_history(&string_list(node, "undo_history"));
    buf.seed_redo(&string_list(node, "redo_history"));
    buf
}

fn string_list(node: &UiNode, prop: &str) -> Vec<String> {
    node.props
        .get(prop)
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn write_buffer(t: &mut crate::UiTree, id: &str, buf: &input_edit::TextBuffer) {
    if let Some(node) = t.get_mut(id) {
        node.props.insert("text".into(), json!(buf.text.clone()));
        node.props.insert("value".into(), json!(buf.text.clone()));
        node.props.insert("caret".into(), json!(buf.caret));
        match buf.anchor {
            Some(a) => {
                node.props.insert("selection_anchor".into(), json!(a));
            }
            None => {
                node.props.remove("selection_anchor");
            }
        }
        node.props
            .insert("undo_history".into(), json!(buf.history()));
        node.props
            .insert("redo_history".into(), json!(buf.redo_history()));
    }
}

/// Apply `f` to the focused field's buffer; `None` when there is no field.
async fn mutate_field(
    tree: &SharedTree,
    nid: &str,
    f: impl FnOnce(&mut input_edit::TextBuffer) -> bool,
) -> Option<FieldState> {
    let focus_id = {
        let t = tree.lock().await;
        t.focused().unwrap_or(nid).to_string()
    };
    mutate_field_id(tree, &focus_id, f).await
}

async fn mutate_field_id(
    tree: &SharedTree,
    id: &str,
    f: impl FnOnce(&mut input_edit::TextBuffer) -> bool,
) -> Option<FieldState> {
    let mut t = tree.lock().await;
    let node = t.get(id)?;
    if !matches!(node.kind.as_str(), "field" | "input") {
        return None;
    }
    let mut buf = buffer_from(node);
    if !f(&mut buf) {
        return None;
    }
    let state = FieldState::from(&buf);
    write_buffer(&mut t, id, &buf);
    t.revision += 1;
    Some(state)
}

async fn sync(tree: &SharedTree) {
    let snapshot = {
        let t = tree.lock().await;
        renderer::serialize_subtree(&t, t.root_id())
    };
    let _ = renderer::sync_tree_to_compositor(&snapshot).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chord_built_from_payload_modifiers() {
        let ev = KeyEvent::from_payload(&json!({ "key": "c", "ctrl": true }));
        assert_eq!(ev.chord(), "Ctrl+C");
        let ev = KeyEvent::from_payload(&json!({ "key": "Tab", "alt": true, "shift": true }));
        assert_eq!(ev.chord(), "Alt+Shift+Tab");
    }

    #[test]
    fn list_navigation_keys_mapped() {
        assert!(matches!(list_nav_delta("ArrowDown"), Some(ListNav::By(1))));
        assert!(matches!(list_nav_delta("ArrowUp"), Some(ListNav::By(-1))));
        assert!(matches!(list_nav_delta("Home"), Some(ListNav::First)));
        assert!(matches!(list_nav_delta("End"), Some(ListNav::Last)));
        assert!(list_nav_delta("q").is_none());
    }

    #[test]
    fn menu_node_is_a_list_primitive() {
        let node = menu_node();
        assert_eq!(node["type"], "list");
        assert_eq!(node["props"]["surface"], "menu");
        assert!(node["props"]["items"].as_array().unwrap().len() >= 4);
    }
}
