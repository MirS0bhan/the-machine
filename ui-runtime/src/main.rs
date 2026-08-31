//! UI Runtime - declarative renderer for the UI State Tree (AUIL) with ASL styling.

mod asl;
mod auil;
mod focus;
mod input_edit;
mod layout;
mod renderer;
mod tokens;
mod widgets;

use common::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

type SharedTree = Arc<Mutex<UiTree>>;

#[derive(Clone, Serialize, Deserialize)]
struct Binding {
    #[serde(rename = "type")]
    kind: String,
    target: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct UiNode {
    id: String,
    #[serde(rename = "type", default = "default_kind")]
    kind: String,
    #[serde(default)]
    props: HashMap<String, serde_json::Value>,
    #[serde(default)]
    children: Vec<String>,
    #[serde(default)]
    asl_style: Option<String>,
    #[serde(default)]
    bindings: Vec<Binding>,
}

fn default_kind() -> String {
    "container".to_string()
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct Theme {
    #[serde(default)]
    colors: HashMap<String, serde_json::Value>,
    #[serde(default)]
    spacing: HashMap<String, serde_json::Value>,
    #[serde(default)]
    rounding: HashMap<String, serde_json::Value>,
    #[serde(default)]
    typography: HashMap<String, serde_json::Value>,
}

impl Theme {
    /// Boot-default dark theme from `docs/design-system/02-style/` (via ASL subset).
    fn design_system_dark() -> Self {
        let doc = asl::parse_asl(asl::design_system_dark_asl());
        let mut colors = HashMap::new();
        for (k, v) in &doc.tokens {
            colors.insert(k.clone(), serde_json::json!(v));
        }
        let mut spacing = HashMap::new();
        if let Some(space) = doc.scales.get("space") {
            for (k, v) in space {
                spacing.insert(k.clone(), serde_json::json!(v));
            }
        }
        spacing.insert("min-target".into(), serde_json::json!(tokens::space::MIN_TARGET));
        let mut rounding = HashMap::new();
        if let Some(radius) = doc.scales.get("radius") {
            for (k, v) in radius {
                rounding.insert(k.clone(), serde_json::json!(v));
            }
        }
        let mut typography = HashMap::new();
        for (k, v) in [
            ("title-2", tokens::type_size::TITLE_2),
            ("body", tokens::type_size::BODY),
            ("caption", tokens::type_size::CAPTION),
            ("label", tokens::type_size::LABEL),
            ("family.default", 0),
        ] {
            if k.starts_with("family") {
                typography.insert(k.into(), serde_json::json!("Inter"));
            } else {
                typography.insert(k.into(), serde_json::json!(v));
            }
        }
        typography.insert("family.numeric".into(), serde_json::json!("JetBrains Mono"));
        Theme {
            colors,
            spacing,
            rounding,
            typography,
        }
    }
}

pub(crate) struct UiTree {
    nodes: HashMap<String, UiNode>,
    root_id: String,
    theme: Theme,
    revision: u64,
    dirty: HashSet<String>,
    /// Currently focused interactive node id.
    focused: Option<String>,
}

impl UiTree {
    fn new() -> Self {
        let mut nodes = HashMap::new();
        nodes.insert(
            "ui.root".to_string(),
            UiNode {
                id: "ui.root".to_string(),
                kind: "container".to_string(),
                props: HashMap::new(),
                children: Vec::new(),
                asl_style: None,
                bindings: Vec::new(),
            },
        );
        UiTree {
            nodes,
            root_id: "ui.root".to_string(),
            theme: Theme::design_system_dark(),
            revision: 1,
            dirty: HashSet::new(),
            focused: None,
        }
    }

    pub(crate) fn get(&self, id: &str) -> Option<&UiNode> {
        self.nodes.get(id)
    }

    pub(crate) fn root_id(&self) -> &str {
        &self.root_id
    }

    pub(crate) fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    pub(crate) fn set_focused(&mut self, id: Option<String>) {
        self.focused = id;
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut UiNode> {
        self.nodes.get_mut(id)
    }

    fn detach(&mut self, id: &str) {
        let parent = self.parent_of(id);
        if let Some(p) = self.nodes.get_mut(&parent) {
            p.children.retain(|c| c != id);
        }
    }

    fn parent_of(&self, id: &str) -> String {
        for (pid, node) in &self.nodes {
            if node.children.iter().any(|c| c == id) {
                return pid.clone();
            }
        }
        self.root_id.clone()
    }

    /// Find the id of the node stored at `anchor` so we can insert relative to it.
    fn resolve_anchor(&self, anchor: &str) -> String {
        if anchor.is_empty() || anchor == "ui.root" {
            return self.root_id.clone();
        }
        if self.nodes.contains_key(anchor) {
            return anchor.to_string();
        }
        // anchor may be "parent.child"; fall back to root
        self.root_id.clone()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    info!("Starting UI Runtime");
    let tree: SharedTree = Arc::new(Mutex::new(UiTree::new()));

    // Load boot AUIL layout if present (G6 — parser embedded in Rust boot path).
    if let Err(e) = load_boot_auil(&tree).await {
        warn!("boot AUIL not loaded: {}", e);
    } else {
        let tree_boot = tree.clone();
        tokio::spawn(async move {
            publish_boot_ready(tree_boot).await;
        });
    }

    // Subscribe to external ui.root changes via state.watch (best-effort).
    {
        let tree = tree.clone();
        tokio::spawn(async move {
            watch_ui_root(tree).await;
        });
    }

    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let socket_path = format!("{}/ui-runtime.sock", socket_dir);
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    info!("UI Runtime listening on {}", socket_path);

    loop {
        let (stream, _) = listener.accept().await?;
        let tree = tree.clone();
        tokio::spawn(async move {
            handle_connection(stream, tree).await;
        });
    }
}

async fn handle_connection(stream: tokio::net::UnixStream, tree: SharedTree) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                if let Ok(response) = process_message(&line, &tree).await {
                    if let Err(e) = writer.write_all(response.as_bytes()).await {
                        error!("Write error: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }
}

async fn process_message(line: &str, tree: &SharedTree) -> anyhow::Result<String> {
    let msg: McpMessage = serde_json::from_str(line.trim())?;
    let id = msg.id;
    let response = match msg.kind {
        MessageKind::Request => {
            let method = msg.method.clone().unwrap_or_default();
            handle_request(method, msg.params, tree).await
        }
        _ => error_response(&id, "E_INVALID_REQUEST", "Only requests supported"),
    };
    Ok(serde_json::to_string(&response)? + "\n")
}

async fn handle_request(
    method: String,
    params: Option<serde_json::Value>,
    tree: &SharedTree,
) -> McpMessage {
    let id = Uuid::new_v4();
    match method.as_str() {
        "ui.patch" => {
            let ops = params
                .and_then(|p| p.get("ops").cloned())
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();
            match apply_patch(tree, ops, true).await {
                Ok(rev) => success_response(&id, serde_json::json!({ "revision": rev })),
                Err(e) => error_response(&id, "E_PATCH_FAILED", &e),
            }
        }
        "ui.get" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let idp = params
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let t = tree.lock().await;
            if let Some(nid) = idp {
                match t.get(&nid) {
                    Some(node) => success_response(
                        &id,
                        serde_json::to_value(node).unwrap_or(serde_json::Value::Null),
                    ),
                    None => error_response(&id, "E_NOT_FOUND", "node not found"),
                }
            } else {
                let root = t.get(&t.root_id).cloned();
                success_response(
                    &id,
                    serde_json::to_value(root).unwrap_or(serde_json::Value::Null),
                )
            }
        }
        "ui.tree" => {
            let t = tree.lock().await;
            let root = t.get(&t.root_id).cloned();
            success_response(
                &id,
                serde_json::to_value(root).unwrap_or(serde_json::Value::Null),
            )
        }
        "ui.bind" => {
            // Register a binding on a node (state:* two-way or mcp: one-way).
            let params = params.unwrap_or(serde_json::Value::Null);
            let nid = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let binding = params.get("binding").cloned();
            let mut t = tree.lock().await;
            if let Some(node) = t.get_mut(&nid) {
                if let Some(b) = binding {
                    if let Ok(b) = serde_json::from_value::<Binding>(b) {
                        node.bindings.push(b);
                        success_response(&id, serde_json::json!({"ok": true}))
                    } else {
                        error_response(&id, "E_INVALID", "bad binding")
                    }
                } else {
                    error_response(&id, "E_INVALID", "missing binding")
                }
            } else {
                error_response(&id, "E_NOT_FOUND", "node not found")
            }
        }
        "ui.event" => {
            // Widget event: execute bindings on the target node.
            let params = params.unwrap_or(serde_json::Value::Null);
            let nid = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let event = params
                .get("event")
                .and_then(|v| v.as_str())
                .unwrap_or("press")
                .to_string();
            let event_payload = params
                .get("payload")
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            // Click focuses interactive widgets.
            if matches!(event.as_str(), "click" | "press") {
                let mut t = tree.lock().await;
                if t.get(&nid)
                    .is_some_and(|n| focus::is_interactive(&n.kind))
                {
                    t.set_focused(Some(nid.clone()));
                }
            }

            // Keyboard editing on the focused field.
            if event == "key" {
                let key = event_payload
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let text = event_payload
                    .get("text")
                    .and_then(|v| v.as_str());
                let mods = input_edit::KeyMods {
                    shift: event_payload
                        .get("shift")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    ctrl: event_payload
                        .get("ctrl")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    alt: event_payload
                        .get("alt")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                    meta: event_payload
                        .get("meta")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                };
                if key == "Tab" {
                    let mut t = tree.lock().await;
                    let next = focus::next_focus(&t, t.focused(), mods.shift);
                    t.set_focused(next.clone());
                    return success_response(
                        &id,
                        serde_json::json!({ "handled": 1, "focused": next, "action": "tab" }),
                    );
                }
                let mut t = tree.lock().await;
                let focus_id = t.focused().unwrap_or(&nid).to_string();
                let edited = if let Some(node) = t.get(&focus_id) {
                    if matches!(node.kind.as_str(), "field" | "input") {
                        let current = node
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
                        let mut buf = input_edit::TextBuffer::from_props(current, caret);
                        if input_edit::apply_key(&mut buf, key, text, &mods) {
                            if let Some(node) = t.get_mut(&focus_id) {
                                node.props
                                    .insert("text".into(), serde_json::json!(buf.text.clone()));
                                node.props
                                    .insert("value".into(), serde_json::json!(buf.text.clone()));
                                node.props
                                    .insert("caret".into(), serde_json::json!(buf.caret));
                            }
                            t.revision += 1;
                            let rev = t.revision;
                            let snapshot = renderer::serialize_subtree(&t, t.root_id());
                            Some((buf, rev, snapshot))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };
                drop(t);
                if let Some((buf, rev, snapshot)) = edited {
                    let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                    return success_response(
                        &id,
                        serde_json::json!({
                            "handled": 1,
                            "action": "edit",
                            "text": buf.text,
                            "caret": buf.caret,
                            "revision": rev,
                        }),
                    );
                }
            }

            let bindings = {
                let t = tree.lock().await;
                let node = t.get(&nid);
                let props = node
                    .map(|n| serde_json::to_value(&n.props).unwrap_or(serde_json::Value::Null))
                    .unwrap_or(serde_json::Value::Null);
                let b = node.map(|n| n.bindings.clone()).unwrap_or_default();
                let chat_text = if event == "press" || event == "click" {
                    t.get("ui.chat_input")
                        .and_then(|n| {
                            n.props
                                .get("text")
                                .or_else(|| n.props.get("value"))
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .map(|s| s.to_string())
                        })
                } else {
                    None
                };
                (b, props, chat_text)
            };
            let mut results = Vec::new();
            for b in &bindings.0 {
                let mut merged = event_payload.clone();
                if let Some(obj) = merged.as_object_mut() {
                    if let Some(pobj) = bindings.1.as_object() {
                        for (k, v) in pobj {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                    if let Some(text) = &bindings.2 {
                        obj.insert("text".into(), serde_json::json!(text));
                    }
                }
                let r = execute_binding(b, &event, &merged).await;
                results.push(serde_json::json!({ "target": b.target, "result": r }));
            }
            success_response(
                &id,
                serde_json::json!({ "handled": results.len(), "results": results }),
            )
        }
        "ui.theme.set" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let theme = params
                .get("theme")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let mut t = tree.lock().await;
            if let Ok(th) = serde_json::from_value::<Theme>(theme) {
                t.theme = th;
                success_response(&id, serde_json::json!({"ok": true}))
            } else {
                error_response(&id, "E_INVALID", "bad theme")
            }
        }
        "ui.theme.get" => {
            let t = tree.lock().await;
            success_response(
                &id,
                serde_json::to_value(&t.theme).unwrap_or(serde_json::Value::Null),
            )
        }
        "ui.status" => {
            let t = tree.lock().await;
            success_response(
                &id,
                serde_json::json!({
                    "status": "running",
                    "revision": t.revision,
                    "nodes": t.nodes.len(),
                    "auil_parser": "rust",
                    "asl_parser": "rust-subset",
                    "focused": t.focused(),
                    "text_stack": "harfbuzz",
                }),
            )
        }
        "ui.focus.get" => {
            let t = tree.lock().await;
            success_response(
                &id,
                serde_json::json!({ "focused": t.focused() }),
            )
        }
        "ui.focus.set" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let nid = params
                .get("id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let mut t = tree.lock().await;
            if let Some(ref idn) = nid {
                if t.get(idn).is_none() {
                    return error_response(&id, "E_NOT_FOUND", "node not found");
                }
            }
            t.set_focused(nid.clone());
            let sync_id = nid.clone();
            drop(t);
            if let Some(sid) = sync_id {
                let _ = mcp_call(
                    "compositor.focus",
                    serde_json::json!({ "id": format!("surface.{sid}") }),
                )
                .await;
            }
            success_response(&id, serde_json::json!({ "focused": nid }))
        }
        "ui.focus.next" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let reverse = params
                .get("reverse")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let mut t = tree.lock().await;
            let next = focus::next_focus(&t, t.focused(), reverse);
            t.set_focused(next.clone());
            drop(t);
            if let Some(ref sid) = next {
                let _ = mcp_call(
                    "compositor.focus",
                    serde_json::json!({ "id": format!("surface.{sid}") }),
                )
                .await;
            }
            success_response(&id, serde_json::json!({ "focused": next }))
        }
        "ui.auil.parse" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let source = params.get("source").and_then(|v| v.as_str()).unwrap_or("");
            match auil::parse_auil(source) {
                Ok(root) => {
                    let ops = auil::auil_to_patch_ops(&root, "ui.root");
                    success_response(
                        &id,
                        serde_json::json!({
                            "root_id": root.id,
                            "ops": ops,
                            "children": root.children.len(),
                        }),
                    )
                }
                Err(e) => error_response(&id, "E_AUIL_PARSE", &e),
            }
        }
        "ui.auil.load" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let source = if let Some(s) = params.get("source").and_then(|v| v.as_str()) {
                s.to_string()
            } else if let Some(path) = params.get("path").and_then(|v| v.as_str()) {
                std::fs::read_to_string(path)
                    .map_err(|e| e.to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            };
            match auil::parse_auil(&source) {
                Ok(root) => {
                    let ops = auil::auil_to_patch_ops(&root, "ui.root");
                    match apply_patch(tree, ops, true).await {
                        Ok(rev) => success_response(
                            &id,
                            serde_json::json!({ "revision": rev, "loaded": true }),
                        ),
                        Err(e) => error_response(&id, "E_PATCH_FAILED", &e),
                    }
                }
                Err(e) => error_response(&id, "E_AUIL_PARSE", &e),
            }
        }
        _ => error_response(&id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
    }
}

fn parse_node(value: &serde_json::Value) -> Result<UiNode, String> {
    serde_json::from_value(value.clone()).map_err(|e| format!("invalid node: {}", e))
}

async fn apply_patch(
    tree: &SharedTree,
    ops: Vec<serde_json::Value>,
    sync_compositor: bool,
) -> Result<u64, String> {
    let mut t = tree.lock().await;
    for op in &ops {
        let kind = op
            .get("op")
            .and_then(|v| v.as_str())
            .or_else(|| op.get("type").and_then(|v| v.as_str()))
            .unwrap_or("");
        match kind {
            "~" | "update" => {
                let nid = op
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or("update missing id")?;
                let props = op
                    .get("props")
                    .and_then(|v| v.as_object())
                    .ok_or("update missing props")?;
                let node = t.get_mut(nid).ok_or("update: node not found")?;
                for (k, v) in props {
                    node.props.insert(k.clone(), v.clone());
                }
                t.dirty.insert(nid.to_string());
            }
            "+" | "insert" => {
                let anchor = op
                    .get("anchor")
                    .and_then(|v| v.as_str())
                    .unwrap_or("ui.root");
                let position = op
                    .get("position")
                    .and_then(|v| v.as_str())
                    .unwrap_or("child");
                let node_val = op.get("node").ok_or("insert missing node")?;
                let node = parse_node(node_val)?;
                let nid = node.id.clone();
                let parent = t.resolve_anchor(anchor);
                if !t.nodes.contains_key(&nid) {
                    t.nodes.insert(nid.clone(), node);
                }
                let p = t.get_mut(&parent).ok_or("insert: anchor not found")?;
                if position == "before" || position == "after" {
                    // simplified: still appended to parent's child list
                }
                if !p.children.contains(&nid) {
                    p.children.push(nid.clone());
                }
                t.dirty.insert(nid);
                t.dirty.insert(parent);
            }
            "-" | "remove" => {
                let nid = op
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or("remove missing id")?;
                // detach from parent and drop subtree
                t.detach(nid);
                remove_subtree(&mut t, nid);
                t.dirty.insert(nid.to_string());
            }
            "!" | "replace" => {
                let _op_id = op
                    .get("id")
                    .and_then(|v| v.as_str())
                    .ok_or("replace missing id")?;
                let node_val = op.get("node").ok_or("replace missing node")?;
                let node = parse_node(node_val)?;
                let nid = node.id.clone();
                let parent = t.parent_of(&nid);
                remove_subtree(&mut t, &nid);
                t.nodes.insert(nid.clone(), node);
                let p = t.get_mut(&parent).ok_or("replace: parent lost")?;
                if !p.children.contains(&nid) {
                    p.children.push(nid.clone());
                }
                t.dirty.insert(nid);
            }
            "@" | "move" => {
                let from = op
                    .get("from")
                    .and_then(|v| v.as_str())
                    .ok_or("move missing from")?;
                let to = op
                    .get("to")
                    .and_then(|v| v.as_str())
                    .ok_or("move missing to")?;
                let parent_from = t.parent_of(from);
                if let Some(p) = t.get_mut(&parent_from) {
                    p.children.retain(|c| c != from);
                }
                let target = t.resolve_anchor(to);
                if let Some(p) = t.get_mut(&target) {
                    if !p.children.contains(&from.to_string()) {
                        p.children.push(from.to_string());
                    }
                }
                t.dirty.insert(from.to_string());
            }
            other => return Err(format!("unknown patch op: {}", other)),
        }
    }
    t.revision += 1;
    let rev = t.revision;
    let root_snapshot = renderer::serialize_subtree(&t, &t.root_id);
    t.dirty.clear();
    drop(t);

    if sync_compositor {
        if let Ok(node) = serde_json::from_value::<UiNode>(root_snapshot.clone()) {
            let _ = reflect_to_state(&node).await;
        }
        let _ = renderer::sync_tree_to_compositor(&root_snapshot).await;
    }
    Ok(rev)
}

async fn watch_ui_root(tree: SharedTree) {
    loop {
        if let Some(val) = mcp_call("state.get", serde_json::json!({ "path": "ui.root" }))
            .await
            .and_then(|v| v.get("value").cloned())
        {
            if let Ok(node) = serde_json::from_value::<UiNode>(val) {
                let mut t = tree.lock().await;
                t.nodes.insert(node.id.clone(), node);
                t.revision += 1;
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

fn remove_subtree(t: &mut UiTree, id: &str) {
    if let Some(node) = t.nodes.remove(id) {
        for c in node.children {
            remove_subtree(t, &c);
        }
    }
}

/// Resolve an ASL token reference like "$colors.primary" against the theme.
/// Wired by ASL mixin application in a future ui-runtime gap (see expansion-proposal Phase 2).
#[allow(dead_code)]
pub(crate) fn resolve_token(token: &str, theme: &Theme) -> serde_json::Value {
    if let Some(rest) = token.strip_prefix('$') {
        let parts: Vec<&str> = rest.split('.').collect();
        let mut cur: serde_json::Value =
            serde_json::to_value(theme).unwrap_or(serde_json::Value::Null);
        for p in parts {
            match cur.get(p) {
                Some(v) => cur = v.clone(),
                None => return serde_json::Value::String(token.to_string()),
            }
        }
        cur
    } else {
        serde_json::Value::String(token.to_string())
    }
}

async fn reflect_to_state(root: &UiNode) -> Option<()> {
    let serialized = serde_json::to_value(root).ok()?;
    mcp_call(
        "state.set",
        serde_json::json!({ "path": "ui.root", "value": serialized }),
    )
    .await?;
    Some(())
}

async fn load_boot_auil(tree: &SharedTree) -> Result<(), String> {
    let path = std::env::var("THE_MACHINE_BOOT_AUIL")
        .unwrap_or_else(|_| "/etc/the-machine/boot.auil".into());
    if !std::path::Path::new(&path).exists() {
        return Err(format!("no boot AUIL at {path}"));
    }
    let source = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let root = auil::parse_auil(&source)?;
    let ops = auil::auil_to_patch_ops(&root, "ui.root");
    apply_patch(tree, ops, false).await?;
    info!("loaded boot AUIL from {}", path);
    Ok(())
}

/// After compositor is up, sync surfaces and wake the agent for the boot greeting.
async fn publish_boot_ready(tree: SharedTree) {
    for _ in 0..30 {
        if compositor_ready().await {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    for attempt in 0..10 {
        let snapshot = {
            let t = tree.lock().await;
            renderer::serialize_subtree(&t, &t.root_id)
        };
        let synced = renderer::sync_tree_to_compositor(&snapshot).await;
        if synced > 0 {
            info!("boot UI synced {} compositor surfaces", synced);
            break;
        }
        warn!("boot compositor sync retry {}", attempt + 1);
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    let _ = mcp_call(
        "event.publish",
        serde_json::json!({
            "category": "boot",
            "pattern": "system.ready",
            "requires_decision": true,
            "payload": {
                "text": "boot greet",
                "summary": "First boot — show hello chat UI"
            }
        }),
    )
    .await;
    info!("published boot.system.ready");
}

async fn compositor_ready() -> bool {
    mcp_call("compositor.status", serde_json::json!({}))
        .await
        .map(|v| {
            v.get("status")
                .and_then(|s| s.as_str())
                .map(|s| s == "running")
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

/// Execute a widget binding: `mcp:method` invokes via bus; `state:path` reads/writes state.
async fn execute_binding(
    binding: &Binding,
    event: &str,
    payload: &serde_json::Value,
) -> Option<serde_json::Value> {
    match binding.kind.as_str() {
        "mcp" => {
            let mut params = serde_json::json!({ "event": event, "payload": payload });
            if binding.target == "policy.confirm" {
                if let Some(cid) = payload.get("correlation_id").and_then(|v| v.as_str()) {
                    params = serde_json::json!({
                        "correlation_id": cid,
                        "approved": payload.get("approved").and_then(|v| v.as_bool()).unwrap_or(false),
                    });
                }
            } else if binding.target == "agent.chat.send" {
                if let Some(text) = payload.get("text").and_then(|v| v.as_str()) {
                    if let Some(obj) = params.as_object_mut() {
                        obj.insert("text".into(), serde_json::json!(text));
                    }
                }
            }
            mcp_call(&binding.target, params).await
        }
        "state" => mcp_call("state.get", serde_json::json!({ "path": binding.target }))
            .await
            .and_then(|v| v.get("value").cloned()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// MCP client helper (talks to the bus).
// ---------------------------------------------------------------------------
async fn mcp_call(method: &str, params: serde_json::Value) -> Option<serde_json::Value> {
    let path = common::bus_socket();
    let stream = tokio::net::UnixStream::connect(&path).await.ok()?;
    let (mut reader, mut writer) = stream.into_split();
    let req = McpMessage::request(Uuid::new_v4(), method, Some(params));
    let mut bytes = serde_json::to_vec(&req).ok()?;
    bytes.push(b'\n');
    writer.write_all(&bytes).await.ok()?;
    writer.flush().await.ok()?;
    let mut buf = vec![0u8; 65536];
    let n = reader.read(&mut buf).await.ok()?;
    if n == 0 {
        return None;
    }
    let resp: serde_json::Value = serde_json::from_slice(&buf[..n]).ok()?;
    resp.get("result").cloned()
}

fn success_response(id: &Uuid, result: serde_json::Value) -> McpMessage {
    McpMessage {
        id: *id,
        stream_id: 0,
        kind: MessageKind::Response,
        method: None,
        params: None,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: &Uuid, code: &str, message: &str) -> McpMessage {
    McpMessage {
        id: *id,
        stream_id: 0,
        kind: MessageKind::Response,
        method: None,
        params: None,
        result: None,
        error: Some(McpError {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tree() -> SharedTree {
        Arc::new(Mutex::new(UiTree::new()))
    }

    #[tokio::test]
    async fn ui_status_reports_running_tree() {
        let tree = test_tree();
        let resp = handle_request("ui.status".into(), None, &tree).await;
        assert!(resp.error.is_none(), "unexpected {:?}", resp.error);
        let result = resp.result.expect("result");
        assert_eq!(
            result.get("status").and_then(|v| v.as_str()),
            Some("running")
        );
        assert_eq!(
            result.get("auil_parser").and_then(|v| v.as_str()),
            Some("rust")
        );
        assert!(result.get("revision").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
        assert!(result.get("nodes").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
    }

    #[tokio::test]
    async fn ui_get_without_id_returns_root_node() {
        let tree = test_tree();
        let resp = handle_request("ui.get".into(), None, &tree).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("result");
        assert_eq!(result.get("id").and_then(|v| v.as_str()), Some("ui.root"));
    }

    #[tokio::test]
    async fn ui_get_unknown_node_is_not_found() {
        let tree = test_tree();
        let resp = handle_request(
            "ui.get".into(),
            Some(serde_json::json!({ "id": "ui.missing" })),
            &tree,
        )
        .await;
        assert_eq!(
            resp.error.as_ref().map(|e| e.code.as_str()),
            Some("E_NOT_FOUND")
        );
    }

    #[tokio::test]
    async fn ui_patch_update_bumps_revision() {
        let tree = test_tree();
        let resp = handle_request(
            "ui.patch".into(),
            Some(serde_json::json!({
                "ops": [{
                    "op": "update",
                    "id": "ui.root",
                    "props": { "label": "boot" }
                }]
            })),
            &tree,
        )
        .await;
        assert!(resp.error.is_none(), "unexpected {:?}", resp.error);
        let revision = resp
            .result
            .as_ref()
            .and_then(|v| v.get("revision"))
            .and_then(|v| v.as_u64())
            .expect("revision");
        assert!(revision >= 2);

        let get_resp = handle_request(
            "ui.get".into(),
            Some(serde_json::json!({ "id": "ui.root" })),
            &tree,
        )
        .await;
        assert_eq!(
            get_resp
                .result
                .as_ref()
                .and_then(|v| v.get("props"))
                .and_then(|p| p.get("label"))
                .and_then(|v| v.as_str()),
            Some("boot")
        );
    }

    #[tokio::test]
    async fn ui_tree_returns_root_snapshot() {
        let tree = test_tree();
        let resp = handle_request("ui.tree".into(), None, &tree).await;
        assert!(resp.error.is_none());
        assert_eq!(
            resp.result
                .as_ref()
                .and_then(|v| v.get("id"))
                .and_then(|v| v.as_str()),
            Some("ui.root")
        );
    }

    #[tokio::test]
    async fn ui_auil_parse_returns_ops_for_simple_layout() {
        let tree = test_tree();
        let source = r#"stack#ui.root dir=v
  text#ui.title(role=title) "Hello"
"#;
        let resp = handle_request(
            "ui.auil.parse".into(),
            Some(serde_json::json!({ "source": source })),
            &tree,
        )
        .await;
        assert!(resp.error.is_none(), "unexpected {:?}", resp.error);
        let result = resp.result.expect("result");
        assert_eq!(
            result.get("root_id").and_then(|v| v.as_str()),
            Some("ui.root")
        );
        let ops = result
            .get("ops")
            .and_then(|v| v.as_array())
            .expect("ops array");
        assert!(!ops.is_empty());
    }

    #[tokio::test]
    async fn unknown_method_returns_not_found() {
        let tree = test_tree();
        let resp = handle_request("ui.nope".into(), None, &tree).await;
        assert_eq!(
            resp.error.as_ref().map(|e| e.code.as_str()),
            Some("E_NOT_FOUND")
        );
    }

    #[test]
    fn resolve_token_reads_theme_color_reference() {
        let mut theme = Theme::default();
        theme
            .colors
            .insert("primary".to_string(), serde_json::json!("#336699"));
        let resolved = resolve_token("$colors.primary", &theme);
        assert_eq!(resolved, serde_json::json!("#336699"));
    }

    #[test]
    fn resolve_token_returns_literal_for_unknown_path() {
        let theme = Theme::default();
        let resolved = resolve_token("$colors.missing", &theme);
        assert_eq!(resolved, serde_json::json!("$colors.missing"));
    }
}
