//! UI Runtime - declarative renderer for the UI State Tree (AUIL) with ASL styling.

mod a11y;
mod asl;
mod atspi;
mod auil;
mod components;
mod dnd;
mod focus;
mod grid;
mod i18n;
mod ime;
mod input_edit;
mod keys;
mod layout;
mod motion;
mod renderer;
mod scroll;
mod shortcuts;
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
pub(crate) struct Binding {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) target: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct UiNode {
    pub(crate) id: String,
    #[serde(rename = "type", default = "default_kind")]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) props: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub(crate) children: Vec<String>,
    #[serde(default)]
    asl_style: Option<String>,
    #[serde(default)]
    pub(crate) bindings: Vec<Binding>,
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
        spacing.insert(
            "min-target".into(),
            serde_json::json!(tokens::space::MIN_TARGET),
        );
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
    pub(crate) nodes: HashMap<String, UiNode>,
    pub(crate) root_id: String,
    theme: Theme,
    pub(crate) revision: u64,
    dirty: HashSet<String>,
    /// Currently focused interactive node id.
    focused: Option<String>,
    drag: Option<dnd::DragSession>,
    /// Dead-key / compose IME state for the focused field.
    pub(crate) ime: ime::ImeState,
    /// Most recent live-region announcement (mirrored to AT clients).
    pub(crate) announcement: Option<String>,
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
            drag: None,
            ime: ime::ImeState::default(),
            announcement: None,
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

    pub(crate) fn get_mut(&mut self, id: &str) -> Option<&mut UiNode> {
        self.nodes.get_mut(id)
    }

    pub(crate) fn detach(&mut self, id: &str) {
        let parent = self.parent_of(id);
        if let Some(p) = self.nodes.get_mut(&parent) {
            p.children.retain(|c| c != id);
        }
    }

    pub(crate) fn parent_of(&self, id: &str) -> String {
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
    i18n::ensure_loaded();
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

    // AT-SPI D-Bus bridge (best-effort; disabled with THE_MACHINE_ATSPI=0).
    {
        let tree_atspi = tree.clone();
        tokio::spawn(async move {
            let _ = atspi::try_start(tree_atspi).await;
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
            let mut nid = params
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

            // Local press feedback — swap chrome before agent bindings.
            // Both `press` (AUIL / agent) and `click` (pointer) activate.
            if matches!(event.as_str(), "click" | "press") {
                let snapshot = {
                    let mut t = tree.lock().await;
                    if t.get(&nid).is_some_and(|n| focus::is_interactive(&n.kind)) {
                        t.set_focused(Some(nid.clone()));
                    }
                    if let Some(node) = t.get_mut(&nid) {
                        if node.kind == "button" {
                            node.props.insert("pressed".into(), serde_json::json!(true));
                        }
                        if node.kind == "toggle" {
                            let on = node
                                .props
                                .get("checked")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            node.props.insert("checked".into(), serde_json::json!(!on));
                        }
                        if node.kind == "slider" {
                            if let Some(geo) = event_payload.get("geometry") {
                                let gx = geo.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as f64;
                                let gw = geo
                                    .get("width")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(1)
                                    .max(1) as f64;
                                let px =
                                    event_payload.get("x").and_then(|v| v.as_i64()).unwrap_or(0)
                                        as f64;
                                let tnorm = ((px - gx) / gw).clamp(0.0, 1.0);
                                let min = node
                                    .props
                                    .get("min")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(0.0);
                                let max = node
                                    .props
                                    .get("max")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(100.0)
                                    .max(min + f64::EPSILON);
                                let value = min + tnorm * (max - min);
                                node.props.insert("value".into(), serde_json::json!(value));
                            }
                        }
                    }
                    // Begin drag outside the get_mut borrow.
                    let start_drag = t.get(&nid).and_then(|node| {
                        if dnd::is_draggable(&node.props) {
                            let x = event_payload
                                .get("x")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i32;
                            let y = event_payload
                                .get("y")
                                .and_then(|v| v.as_i64())
                                .unwrap_or(0) as i32;
                            let payload = serde_json::json!({
                                "id": nid,
                                "label": node.props.get("label").cloned().unwrap_or(serde_json::Value::Null),
                            });
                            Some(dnd::DragSession::begin(&nid, x, y, payload))
                        } else {
                            None
                        }
                    });
                    if let Some(session) = start_drag {
                        t.drag = Some(session);
                    }
                    t.revision += 1;
                    renderer::serialize_subtree(&t, t.root_id())
                };
                let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                // Kick opacity tween on the pressed surface.
                let _ = mcp_call(
                    "compositor.surface",
                    serde_json::json!({
                        "action": "update",
                        "id": format!("surface.{nid}"),
                        "opacity": 0.85,
                        "opacity_target": 1.0,
                        "motion_ms": motion::SNAPPY.duration_ms,
                    }),
                )
                .await;
            }
            if event == "release" {
                let drop = {
                    let mut t = tree.lock().await;
                    if let Some(node) = t.get_mut(&nid) {
                        node.props
                            .insert("pressed".into(), serde_json::json!(false));
                    }
                    let ended = t.drag.take().map(|d| d.end_payload(Some(&nid)));
                    let snapshot = renderer::serialize_subtree(&t, t.root_id());
                    (ended, snapshot)
                };
                let _ = renderer::sync_tree_to_compositor(&drop.1).await;
                if let Some(payload) = drop.0 {
                    // Fire change bindings on drop target if any.
                    let bindings = {
                        let t = tree.lock().await;
                        t.get(&nid).map(|n| n.bindings.clone()).unwrap_or_default()
                    };
                    let mut results = Vec::new();
                    for b in &bindings {
                        let r = execute_binding(b, "change", &payload).await;
                        results.push(serde_json::json!({ "target": b.target, "result": r }));
                    }
                    return success_response(
                        &id,
                        serde_json::json!({
                            "handled": 1,
                            "action": "drop",
                            "payload": payload,
                            "results": results,
                        }),
                    );
                }
            }

            // Hover / drag move.
            if event == "move" || event == "drag" {
                let x = event_payload.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = event_payload.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dragging = {
                    let mut t = tree.lock().await;
                    if let Some(ref mut drag) = t.drag {
                        drag.move_to(x, y);
                        Some(drag.end_payload(Some(&nid)))
                    } else {
                        None
                    }
                };
                if let Some(payload) = dragging {
                    return success_response(
                        &id,
                        serde_json::json!({ "handled": 1, "action": "drag", "payload": payload }),
                    );
                }
                let snapshot = {
                    let mut t = tree.lock().await;
                    for node in t.nodes.values_mut() {
                        if node.props.get("hovered").and_then(|v| v.as_bool()) == Some(true) {
                            node.props
                                .insert("hovered".into(), serde_json::json!(false));
                        }
                    }
                    if let Some(node) = t.get_mut(&nid) {
                        if focus::is_interactive(&node.kind) {
                            node.props.insert("hovered".into(), serde_json::json!(true));
                        }
                    }
                    renderer::serialize_subtree(&t, t.root_id())
                };
                let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                return success_response(
                    &id,
                    serde_json::json!({ "handled": 1, "action": "hover" }),
                );
            }

            // Wheel → scroll focused list / overflow container.
            if event == "wheel" {
                let dy = event_payload
                    .get("delta_y")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let (snapshot, glide_target, offset) = {
                    let mut t = tree.lock().await;
                    let target = t
                        .focused()
                        .map(|s| s.to_string())
                        .filter(|id| t.get(id).is_some_and(|n| n.kind == "list"))
                        .unwrap_or_else(|| nid.clone());
                    let mut glide = None;
                    let mut offset = None;
                    if let Some(node) = t.get_mut(&target) {
                        if node.kind == "list" {
                            let vh = node
                                .props
                                .get("height")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(160) as u32;
                            scroll::apply_wheel(&mut node.props, dy, vh);
                            offset = node.props.get("scroll_y").cloned();
                            glide = Some((target.clone(), vh));
                            t.revision += 1;
                        }
                    }
                    (renderer::serialize_subtree(&t, t.root_id()), glide, offset)
                };
                let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                if let Some((target, vh)) = glide_target {
                    spawn_glide(tree.clone(), target, vh);
                }
                return success_response(
                    &id,
                    serde_json::json!({ "handled": 1, "action": "scroll", "scroll_y": offset }),
                );
            }

            // Keyboard: shortcut table first, then IME, then field/list editing.
            if event == "key" {
                match keys::handle(tree, &nid, &event_payload).await {
                    keys::KeyOutcome::Handled(result) => {
                        return success_response(&id, result);
                    }
                    keys::KeyOutcome::Activate(target) => {
                        // Enter on a focused control runs its bindings as a press.
                        nid = target;
                        return activate_node(tree, &nid, &event_payload, &id).await;
                    }
                    keys::KeyOutcome::Pass => {}
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
                    t.get("ui.chat_input").and_then(|n| {
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
            // Clear local press after bindings fire.
            if matches!(event.as_str(), "click" | "press") {
                let snapshot = {
                    let mut t = tree.lock().await;
                    if let Some(node) = t.get_mut(&nid) {
                        node.props
                            .insert("pressed".into(), serde_json::json!(false));
                    }
                    renderer::serialize_subtree(&t, t.root_id())
                };
                let _ = renderer::sync_tree_to_compositor(&snapshot).await;
            }
            success_response(
                &id,
                serde_json::json!({ "handled": results.len(), "results": results }),
            )
        }
        "ui.shortcuts.list" => success_response(&id, shortcuts::list()),
        "ui.shortcuts.set" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let chord = params.get("chord").and_then(|v| v.as_str()).unwrap_or("");
            let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let method = params.get("method").and_then(|v| v.as_str());
            let description = params
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            match shortcuts::set(chord, action, method, description) {
                Ok(normalized) => success_response(
                    &id,
                    serde_json::json!({ "ok": true, "chord": normalized, "action": action }),
                ),
                Err(e) => error_response(&id, "E_INVALID", &e),
            }
        }
        "ui.shortcuts.reset" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            match params.get("chord").and_then(|v| v.as_str()) {
                Some(chord) if !chord.is_empty() => {
                    let removed = shortcuts::unset(chord);
                    success_response(
                        &id,
                        serde_json::json!({ "ok": true, "removed": removed, "chord": chord }),
                    )
                }
                _ => {
                    shortcuts::reset();
                    success_response(&id, shortcuts::list())
                }
            }
        }
        "ui.snapshot" => {
            let t = tree.lock().await;
            success_response(&id, snapshot_value(&t))
        }
        "ui.menu.open" => {
            let ops = vec![serde_json::json!({
                "op": "insert",
                "anchor": "ui.root",
                "position": "child",
                "node": keys::menu_node(),
            })];
            match apply_patch(tree, ops, true).await {
                Ok(rev) => {
                    {
                        let mut t = tree.lock().await;
                        t.set_focused(Some(keys::MENU_ID.to_string()));
                    }
                    success_response(
                        &id,
                        serde_json::json!({ "id": keys::MENU_ID, "revision": rev }),
                    )
                }
                Err(e) => error_response(&id, "E_PATCH_FAILED", &e),
            }
        }
        "ui.select.all" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let target = {
                let t = tree.lock().await;
                params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| t.focused().map(|s| s.to_string()))
            };
            let Some(target) = target else {
                return error_response(&id, "E_INVALID", "no focused node");
            };
            let mut t = tree.lock().await;
            let Some(node) = t.get(&target) else {
                return error_response(&id, "E_NOT_FOUND", "node not found");
            };
            if !matches!(node.kind.as_str(), "field" | "input") {
                return error_response(&id, "E_INVALID", "node is not a text field");
            }
            let mut buf = keys::buffer_from(node);
            buf.select_all();
            let selection = buf.selection();
            keys::write_buffer(&mut t, &target, &buf);
            t.revision += 1;
            let snapshot = renderer::serialize_subtree(&t, t.root_id());
            drop(t);
            let _ = renderer::sync_tree_to_compositor(&snapshot).await;
            success_response(
                &id,
                serde_json::json!({
                    "id": target,
                    "selection": selection.map(|(a, b)| [a, b]),
                }),
            )
        }
        "ui.scroll" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let target = {
                let t = tree.lock().await;
                params
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| t.focused().map(|s| s.to_string()))
            };
            let Some(target) = target else {
                return error_response(&id, "E_INVALID", "no target node");
            };
            let mut t = tree.lock().await;
            if t.get(&target).is_none() {
                return error_response(&id, "E_NOT_FOUND", "node not found");
            }
            let viewport = t
                .get(&target)
                .and_then(|n| n.props.get("height"))
                .and_then(|v| v.as_u64())
                .unwrap_or(160) as u32;
            let node = t.get_mut(&target).expect("target exists");
            if let Some(offset) = params.get("offset_y").and_then(|v| v.as_i64()) {
                scroll::scroll_to(&mut node.props, offset as i32, viewport);
            } else if let Some(pages) = params.get("pages").and_then(|v| v.as_i64()) {
                scroll::apply_page(&mut node.props, pages as i32, viewport);
            } else if let Some(delta) = params.get("delta_y").and_then(|v| v.as_i64()) {
                scroll::apply_wheel(&mut node.props, delta as i32, viewport);
            } else if let Some(index) = params.get("ensure_visible").and_then(|v| v.as_u64()) {
                scroll::ensure_visible(&mut node.props, index as usize, viewport);
            } else {
                return error_response(
                    &id,
                    "E_INVALID",
                    "one of offset_y / pages / delta_y / ensure_visible required",
                );
            }
            let offset = node.props.get("scroll_y").cloned();
            let max = node.props.get("scroll_max").cloned();
            t.revision += 1;
            let snapshot = renderer::serialize_subtree(&t, t.root_id());
            drop(t);
            let _ = renderer::sync_tree_to_compositor(&snapshot).await;
            success_response(
                &id,
                serde_json::json!({ "id": target, "scroll_y": offset, "scroll_max": max }),
            )
        }
        "ui.a11y.announce" => {
            // Live-region announcement: recorded on the node, mirrored to
            // AT-SPI listeners, and surfaced in ui.a11y.tree.
            let params = params.unwrap_or(serde_json::Value::Null);
            let message = params
                .get("message")
                .or_else(|| params.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if message.is_empty() {
                return error_response(&id, "E_INVALID", "message required");
            }
            let target = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("ui.activity")
                .to_string();
            let politeness = params
                .get("live")
                .and_then(|v| v.as_str())
                .unwrap_or("polite")
                .to_string();
            let snapshot = {
                let mut t = tree.lock().await;
                if let Some(node) = t.get_mut(&target) {
                    node.props.insert("text".into(), serde_json::json!(message));
                    node.props
                        .insert("live".into(), serde_json::json!(politeness.clone()));
                }
                t.announcement = Some(message.to_string());
                t.revision += 1;
                renderer::serialize_subtree(&t, t.root_id())
            };
            let _ = renderer::sync_tree_to_compositor(&snapshot).await;
            atspi::announce(message, &politeness).await;
            success_response(
                &id,
                serde_json::json!({
                    "announced": message,
                    "id": target,
                    "live": politeness,
                    "atspi": atspi::is_running(),
                }),
            )
        }
        "ui.a11y.focus_order" => {
            let t = tree.lock().await;
            let order: Vec<serde_json::Value> = focus::focus_order(&t)
                .into_iter()
                .enumerate()
                .map(|(i, (nid, role, name))| {
                    serde_json::json!({
                        "index": i,
                        "id": nid,
                        "role": role,
                        "name": name,
                    })
                })
                .collect();
            success_response(
                &id,
                serde_json::json!({
                    "order": order,
                    "focused": t.focused(),
                    "trapped": t.nodes.values().any(|n| n.kind == "dialog"),
                }),
            )
        }
        "ui.workspace.clear" => {
            // Workspace lifecycle: drop every agent-placed control under the
            // anchor. `keep_hint` preserves the caption that explains the area.
            let params = params.unwrap_or(serde_json::Value::Null);
            let anchor = params
                .get("anchor")
                .and_then(|v| v.as_str())
                .unwrap_or("ui.workspace")
                .to_string();
            let keep_hint = params
                .get("keep_hint")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let hint_id = format!("{anchor}_hint");
            let (removed, snapshot) = {
                let mut t = tree.lock().await;
                if t.get(&anchor).is_none() {
                    return error_response(&id, "E_NOT_FOUND", "anchor not found");
                }
                let children: Vec<String> = t
                    .get(&anchor)
                    .map(|n| n.children.clone())
                    .unwrap_or_default();
                let mut removed = Vec::new();
                for child in children {
                    if keep_hint && child == hint_id {
                        continue;
                    }
                    t.detach(&child);
                    remove_subtree(&mut t, &child);
                    removed.push(child);
                }
                if keep_hint {
                    if let Some(hint) = t.get_mut(&hint_id) {
                        hint.props.insert(
                            "text".into(),
                            serde_json::json!("Workspace cleared — ask for a control"),
                        );
                    }
                }
                if t.focused()
                    .map(|f| !t.nodes.contains_key(f))
                    .unwrap_or(false)
                {
                    t.set_focused(None);
                }
                t.revision += 1;
                let snapshot = renderer::serialize_subtree(&t, t.root_id());
                (removed, snapshot)
            };
            let _ = renderer::sync_tree_to_compositor(&snapshot).await;
            for rid in &removed {
                let _ = mcp_call(
                    "compositor.surface",
                    serde_json::json!({
                        "action": "destroy",
                        "id": format!("surface.{rid}"),
                    }),
                )
                .await;
            }
            success_response(
                &id,
                serde_json::json!({
                    "cleared": removed.len(),
                    "removed": removed,
                    "anchor": anchor,
                }),
            )
        }
        "ui.workspace.replace" => {
            // Clear then load: the caller supplies AUIL source or patch ops.
            let params = params.unwrap_or(serde_json::Value::Null);
            let anchor = params
                .get("anchor")
                .and_then(|v| v.as_str())
                .unwrap_or("ui.workspace")
                .to_string();
            {
                let mut t = tree.lock().await;
                if t.get(&anchor).is_none() {
                    return error_response(&id, "E_NOT_FOUND", "anchor not found");
                }
                let children: Vec<String> = t
                    .get(&anchor)
                    .map(|n| n.children.clone())
                    .unwrap_or_default();
                for child in children {
                    t.detach(&child);
                    remove_subtree(&mut t, &child);
                }
                t.revision += 1;
            }
            let ops = if let Some(source) = params.get("source").and_then(|v| v.as_str()) {
                match auil::parse_auil(source) {
                    Ok(root) => auil::auil_to_patch_ops(&root, &anchor),
                    Err(e) => return error_response(&id, "E_AUIL_PARSE", &e),
                }
            } else {
                params
                    .get("ops")
                    .and_then(|v| v.as_array().cloned())
                    .unwrap_or_default()
            };
            let count = ops.len();
            match apply_patch(tree, ops, true).await {
                Ok(rev) => success_response(
                    &id,
                    serde_json::json!({ "revision": rev, "ops": count, "anchor": anchor }),
                ),
                Err(e) => error_response(&id, "E_PATCH_FAILED", &e),
            }
        }
        "ui.workspace.list" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let anchor = params
                .get("anchor")
                .and_then(|v| v.as_str())
                .unwrap_or("ui.workspace");
            let t = tree.lock().await;
            match t.get(anchor) {
                Some(node) => {
                    let controls: Vec<serde_json::Value> = node
                        .children
                        .iter()
                        .filter_map(|cid| t.get(cid))
                        .map(|n| {
                            serde_json::json!({
                                "id": n.id,
                                "kind": n.kind,
                                "label": a11y::name_for(&n.kind, &n.id, &n.props),
                                "surface": n.props.get("surface"),
                            })
                        })
                        .collect();
                    success_response(
                        &id,
                        serde_json::json!({ "anchor": anchor, "controls": controls }),
                    )
                }
                None => error_response(&id, "E_NOT_FOUND", "anchor not found"),
            }
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
                    "text_stack": "harfrust",
                    "a11y": if atspi::is_running() { "mcp-tree+atspi-dbus" } else { "mcp-tree" },
                    "atspi": atspi::status(),
                    "i18n": i18n::status(),
                    "ime": "compose-deadkey",
                    "motion": "snappy|gentle|reduced",
                    "components": components::catalog().len(),
                }),
            )
        }
        "ui.a11y.tree" => {
            let t = tree.lock().await;
            success_response(&id, a11y::serialize_tree(&t))
        }
        "ui.atspi.status" => success_response(&id, atspi::status()),
        "ui.i18n.status" => success_response(&id, i18n::status()),
        "ui.i18n.t" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let key = params.get("key").and_then(|v| v.as_str()).unwrap_or("");
            if key.is_empty() {
                error_response(&id, "E_INVALID", "key required")
            } else {
                success_response(
                    &id,
                    serde_json::json!({
                        "key": key,
                        "text": i18n::t(key),
                        "locale": i18n::current_locale(),
                    }),
                )
            }
        }
        "ui.i18n.load" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let locale = params.get("locale").and_then(|v| v.as_str()).unwrap_or("");
            if locale.is_empty() {
                error_response(&id, "E_INVALID", "locale required")
            } else {
                match i18n::load_locale(locale) {
                    Ok(()) => success_response(&id, i18n::status()),
                    Err(e) => error_response(&id, "E_I18N", &e),
                }
            }
        }
        "ui.components.list" => success_response(
            &id,
            serde_json::json!({ "components": components::catalog() }),
        ),
        "ui.focus.get" => {
            let t = tree.lock().await;
            success_response(&id, serde_json::json!({ "focused": t.focused() }))
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

/// Run a node's bindings as a press, including the press/release chrome.
///
/// Shared by pointer clicks and by Enter on a focused control so both paths
/// deliver the same payload (node props plus the current chat field text).
async fn activate_node(
    tree: &SharedTree,
    nid: &str,
    event_payload: &serde_json::Value,
    id: &Uuid,
) -> McpMessage {
    let snapshot = {
        let mut t = tree.lock().await;
        if let Some(node) = t.get_mut(nid) {
            node.props.insert("pressed".into(), serde_json::json!(true));
        }
        renderer::serialize_subtree(&t, t.root_id())
    };
    let _ = renderer::sync_tree_to_compositor(&snapshot).await;

    let (bindings, props, chat_text) = collect_binding_context(tree, nid, "press").await;
    let mut results = Vec::new();
    for b in &bindings {
        let merged = merge_payload(event_payload, &props, chat_text.as_deref());
        let r = execute_binding(b, "press", &merged).await;
        results.push(serde_json::json!({ "target": b.target, "result": r }));
    }

    let snapshot = {
        let mut t = tree.lock().await;
        if let Some(node) = t.get_mut(nid) {
            node.props
                .insert("pressed".into(), serde_json::json!(false));
        }
        renderer::serialize_subtree(&t, t.root_id())
    };
    let _ = renderer::sync_tree_to_compositor(&snapshot).await;
    success_response(
        id,
        serde_json::json!({
            "handled": results.len(),
            "results": results,
            "action": "activate",
        }),
    )
}

async fn collect_binding_context(
    tree: &SharedTree,
    nid: &str,
    event: &str,
) -> (Vec<Binding>, serde_json::Value, Option<String>) {
    let t = tree.lock().await;
    let node = t.get(nid);
    let props = node
        .map(|n| serde_json::to_value(&n.props).unwrap_or(serde_json::Value::Null))
        .unwrap_or(serde_json::Value::Null);
    let bindings = node.map(|n| n.bindings.clone()).unwrap_or_default();
    let chat_text = if matches!(event, "press" | "click" | "activate" | "submit") {
        t.get("ui.chat_input").and_then(|n| {
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
    (bindings, props, chat_text)
}

fn merge_payload(
    event_payload: &serde_json::Value,
    props: &serde_json::Value,
    chat_text: Option<&str>,
) -> serde_json::Value {
    let mut merged = event_payload.clone();
    if !merged.is_object() {
        merged = serde_json::json!({});
    }
    if let Some(obj) = merged.as_object_mut() {
        if let Some(pobj) = props.as_object() {
            for (k, v) in pobj {
                obj.insert(k.clone(), v.clone());
            }
        }
        if let Some(text) = chat_text {
            obj.insert("text".into(), serde_json::json!(text));
        }
    }
    merged
}

/// Frame interval used to decay scroll momentum (~60 Hz).
const GLIDE_TICK_MS: u64 = 16;

/// Upper bound on glide frames so a runaway velocity cannot spin forever.
const GLIDE_MAX_TICKS: u32 = 240;

/// Keep a flicked list gliding after the last wheel event.
///
/// Each tick decays `scroll_velocity` and re-syncs the subtree, so momentum is a
/// real animation rather than a prop nobody reads.
fn spawn_glide(tree: SharedTree, target: String, viewport_h: u32) {
    tokio::spawn(async move {
        for _ in 0..GLIDE_MAX_TICKS {
            tokio::time::sleep(std::time::Duration::from_millis(GLIDE_TICK_MS)).await;
            let (more, snapshot) = {
                let mut t = tree.lock().await;
                let Some(node) = t.get_mut(&target) else {
                    return;
                };
                let more = scroll::settle(&mut node.props, viewport_h);
                t.revision += 1;
                (more, renderer::serialize_subtree(&t, t.root_id()))
            };
            let _ = renderer::sync_tree_to_compositor(&snapshot).await;
            if !more {
                return;
            }
        }
    });
}

/// Text rendering of the current tree, used by `ui.snapshot` and PrintScreen.
pub(crate) fn snapshot_value(t: &UiTree) -> serde_json::Value {
    fn walk(t: &UiTree, id: &str, depth: usize, out: &mut Vec<String>) {
        if let Some(node) = t.get(id) {
            let name = a11y::name_for(&node.kind, &node.id, &node.props);
            out.push(format!(
                "{}{} [{}] {}",
                "  ".repeat(depth),
                node.id,
                node.kind,
                name
            ));
            for child in &node.children {
                walk(t, child, depth + 1, out);
            }
        }
    }
    let mut lines = Vec::new();
    walk(t, t.root_id(), 0, &mut lines);
    let (w, h) = layout::default_viewport();
    serde_json::json!({
        "revision": t.revision,
        "nodes": t.nodes.len(),
        "focused": t.focused(),
        "viewport": { "width": w, "height": h },
        "text": lines.join("\n"),
    })
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
                // Re-inserting an existing id updates it in place instead of
                // silently conflicting: the agent can respawn a control and the
                // workspace keeps one coherent node per id.
                if let Some(existing) = t.nodes.get_mut(&nid) {
                    existing.kind = node.kind;
                    for (k, v) in node.props {
                        existing.props.insert(k, v);
                    }
                    if !node.bindings.is_empty() {
                        existing.bindings = node.bindings;
                    }
                    if node.asl_style.is_some() {
                        existing.asl_style = node.asl_style;
                    }
                    let current_parent = t.parent_of(&nid);
                    if current_parent != parent {
                        t.detach(&nid);
                    }
                } else {
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

pub(crate) fn remove_subtree(t: &mut UiTree, id: &str) {
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
pub(crate) async fn mcp_call(method: &str, params: serde_json::Value) -> Option<serde_json::Value> {
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

    async fn seed_workspace(tree: &SharedTree, kinds: &[(&str, &str)]) {
        let mut ops = vec![serde_json::json!({
            "op": "insert",
            "anchor": "ui.root",
            "node": { "id": "ui.workspace", "type": "stack", "props": { "dir": "v" } }
        })];
        ops.push(serde_json::json!({
            "op": "insert",
            "anchor": "ui.workspace",
            "node": { "id": "ui.workspace_hint", "type": "text", "props": { "text": "hint" } }
        }));
        for (id, kind) in kinds {
            ops.push(serde_json::json!({
                "op": "insert",
                "anchor": "ui.workspace",
                "node": { "id": id, "type": kind, "props": { "label": id } }
            }));
        }
        apply_patch(tree, ops, false).await.expect("seed");
    }

    #[tokio::test]
    async fn insert_with_existing_id_updates_in_place() {
        let tree = test_tree();
        seed_workspace(&tree, &[("ui.agent_button_1", "button")]).await;
        apply_patch(
            &tree,
            vec![serde_json::json!({
                "op": "insert",
                "anchor": "ui.workspace",
                "node": {
                    "id": "ui.agent_button_1",
                    "type": "toggle",
                    "props": { "label": "replaced", "checked": true },
                    "bindings": [{ "type": "mcp", "target": "ui.status" }]
                }
            })],
            false,
        )
        .await
        .expect("respawn");
        let t = tree.lock().await;
        let node = t.get("ui.agent_button_1").expect("node");
        assert_eq!(node.kind, "toggle");
        assert_eq!(
            node.props.get("label").and_then(|v| v.as_str()),
            Some("replaced")
        );
        assert_eq!(node.bindings.len(), 1);
        let workspace = t.get("ui.workspace").expect("workspace");
        assert_eq!(
            workspace
                .children
                .iter()
                .filter(|c| *c == "ui.agent_button_1")
                .count(),
            1,
            "duplicate insert must not duplicate the child entry"
        );
    }

    #[tokio::test]
    async fn workspace_clear_removes_controls_and_keeps_hint() {
        let tree = test_tree();
        seed_workspace(
            &tree,
            &[("ui.agent_button_1", "button"), ("ui.agent_list_2", "list")],
        )
        .await;
        let resp = handle_request("ui.workspace.clear".into(), None, &tree).await;
        assert!(resp.error.is_none(), "unexpected {:?}", resp.error);
        assert_eq!(
            resp.result
                .as_ref()
                .and_then(|v| v.get("cleared"))
                .and_then(|v| v.as_u64()),
            Some(2)
        );
        let t = tree.lock().await;
        assert!(t.get("ui.agent_button_1").is_none());
        assert!(t.get("ui.agent_list_2").is_none());
        assert!(t.get("ui.workspace_hint").is_some());
    }

    #[tokio::test]
    async fn workspace_clear_can_drop_the_hint_too() {
        let tree = test_tree();
        seed_workspace(&tree, &[("ui.agent_button_1", "button")]).await;
        let resp = handle_request(
            "ui.workspace.clear".into(),
            Some(serde_json::json!({ "keep_hint": false })),
            &tree,
        )
        .await;
        assert!(resp.error.is_none());
        let t = tree.lock().await;
        assert!(t.get("ui.workspace_hint").is_none());
        assert_eq!(t.get("ui.workspace").unwrap().children.len(), 0);
    }

    #[tokio::test]
    async fn workspace_clear_unknown_anchor_is_not_found() {
        let tree = test_tree();
        let resp = handle_request(
            "ui.workspace.clear".into(),
            Some(serde_json::json!({ "anchor": "ui.nope" })),
            &tree,
        )
        .await;
        assert_eq!(
            resp.error.as_ref().map(|e| e.code.as_str()),
            Some("E_NOT_FOUND")
        );
    }

    #[tokio::test]
    async fn workspace_replace_loads_auil_under_anchor() {
        let tree = test_tree();
        seed_workspace(&tree, &[("ui.agent_button_1", "button")]).await;
        let resp = handle_request(
            "ui.workspace.replace".into(),
            Some(serde_json::json!({
                "source": "stack#ui.pack dir=v\n  button#ui.new_action label=Go on:press=mcp:agent.status"
            })),
            &tree,
        )
        .await;
        assert!(resp.error.is_none(), "unexpected {:?}", resp.error);
        let t = tree.lock().await;
        assert!(t.get("ui.agent_button_1").is_none());
        assert!(t.get("ui.new_action").is_some());
    }

    #[tokio::test]
    async fn workspace_list_reports_controls() {
        let tree = test_tree();
        seed_workspace(
            &tree,
            &[
                ("ui.agent_chart_1", "chart"),
                ("ui.agent_toggle_2", "toggle"),
            ],
        )
        .await;
        let resp = handle_request("ui.workspace.list".into(), None, &tree).await;
        let controls = resp
            .result
            .as_ref()
            .and_then(|v| v.get("controls"))
            .and_then(|v| v.as_array())
            .cloned()
            .expect("controls");
        let kinds: Vec<&str> = controls
            .iter()
            .filter_map(|c| c.get("kind").and_then(|v| v.as_str()))
            .collect();
        assert!(kinds.contains(&"chart"));
        assert!(kinds.contains(&"toggle"));
    }

    async fn seed_shell(tree: &SharedTree) {
        let src = std::fs::read_to_string(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build/boot.auil"),
        )
        .expect("boot.auil");
        let root = auil::parse_auil(&src).expect("parse");
        let ops = auil::auil_to_patch_ops(&root, "ui.root");
        apply_patch(tree, ops, false).await.expect("seed shell");
    }

    async fn key(tree: &SharedTree, id: &str, payload: serde_json::Value) -> serde_json::Value {
        let resp = handle_request(
            "ui.event".into(),
            Some(serde_json::json!({ "id": id, "event": "key", "payload": payload })),
            tree,
        )
        .await;
        assert!(resp.error.is_none(), "unexpected {:?}", resp.error);
        resp.result.expect("result")
    }

    async fn focus(tree: &SharedTree, id: &str) {
        let mut t = tree.lock().await;
        t.set_focused(Some(id.to_string()));
    }

    async fn field_text(tree: &SharedTree, id: &str) -> String {
        let t = tree.lock().await;
        t.get(id)
            .and_then(|n| n.props.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    }

    async fn type_text(tree: &SharedTree, id: &str, text: &str) {
        for ch in text.chars() {
            key(
                tree,
                id,
                serde_json::json!({ "key": ch.to_string(), "text": ch.to_string() }),
            )
            .await;
        }
    }

    #[tokio::test]
    async fn arrow_keys_move_the_caret_without_editing() {
        let tree = test_tree();
        seed_shell(&tree).await;
        focus(&tree, "ui.chat_input").await;
        type_text(&tree, "ui.chat_input", "hello").await;
        for k in ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"] {
            let out = key(&tree, "ui.chat_input", serde_json::json!({ "key": k })).await;
            assert_eq!(out["action"], "caret", "key {k}");
            assert_eq!(out["text"], "hello", "key {k} must not edit text");
        }
    }

    #[tokio::test]
    async fn home_end_page_and_delete_keys_are_handled() {
        let tree = test_tree();
        seed_shell(&tree).await;
        focus(&tree, "ui.chat_input").await;
        type_text(&tree, "ui.chat_input", "abcdef").await;
        let out = key(&tree, "ui.chat_input", serde_json::json!({ "key": "Home" })).await;
        assert_eq!(out["caret"], 0);
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "Delete" }),
        )
        .await;
        assert_eq!(out["text"], "bcdef");
        let out = key(&tree, "ui.chat_input", serde_json::json!({ "key": "End" })).await;
        assert_eq!(out["caret"], 5);
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "PageUp" }),
        )
        .await;
        assert_eq!(out["caret"], 0);
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "PageDown" }),
        )
        .await;
        assert_eq!(out["caret"], 5);
    }

    #[tokio::test]
    async fn shift_arrow_selects_and_ctrl_a_selects_all() {
        let tree = test_tree();
        seed_shell(&tree).await;
        focus(&tree, "ui.chat_input").await;
        type_text(&tree, "ui.chat_input", "hello").await;
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "ArrowLeft", "shift": true }),
        )
        .await;
        assert_eq!(out["selection"], serde_json::json!([4, 5]));
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "a", "ctrl": true }),
        )
        .await;
        assert_eq!(out["action"], "select.all");
        assert_eq!(out["selection"], serde_json::json!([0, 5]));
    }

    #[tokio::test]
    async fn ctrl_z_undoes_and_ctrl_shift_z_redoes() {
        let tree = test_tree();
        seed_shell(&tree).await;
        focus(&tree, "ui.chat_input").await;
        type_text(&tree, "ui.chat_input", "abc").await;
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "z", "ctrl": true }),
        )
        .await;
        assert_eq!(out["action"], "undo");
        assert_eq!(out["text"], "ab");
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "z", "ctrl": true, "shift": true }),
        )
        .await;
        assert_eq!(out["action"], "redo");
        assert_eq!(out["text"], "abc");
    }

    #[tokio::test]
    async fn undo_with_empty_history_reports_nothing_to_undo() {
        let tree = test_tree();
        seed_shell(&tree).await;
        focus(&tree, "ui.chat_input").await;
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "z", "ctrl": true }),
        )
        .await;
        assert_eq!(out["handled"], 0);
        assert_eq!(out["reason"], "nothing to undo");
    }

    #[tokio::test]
    async fn copy_without_selection_or_clipboard_fails_soft() {
        let tree = test_tree();
        seed_shell(&tree).await;
        focus(&tree, "ui.chat_input").await;
        // Empty field: nothing to copy, and the response says so.
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "c", "ctrl": true }),
        )
        .await;
        assert_eq!(out["handled"], 0);
        assert_eq!(out["reason"], "nothing selected");
        // No bus in unit tests: paste must report unavailable, never forge text.
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "v", "ctrl": true }),
        )
        .await;
        assert_eq!(out["handled"], 0);
        assert_eq!(out["action"], "clipboard.paste");
        assert_eq!(field_text(&tree, "ui.chat_input").await, "");
    }

    #[tokio::test]
    async fn paste_into_non_editable_node_is_refused() {
        let tree = test_tree();
        seed_shell(&tree).await;
        focus(&tree, "ui.chat_send").await;
        let out = key(
            &tree,
            "ui.chat_send",
            serde_json::json!({ "key": "v", "ctrl": true }),
        )
        .await;
        assert_eq!(out["handled"], 0);
        assert_eq!(out["reason"], "focused node is not editable");
    }

    #[tokio::test]
    async fn tab_and_shift_tab_walk_focus_both_ways() {
        let tree = test_tree();
        seed_shell(&tree).await;
        focus(&tree, "ui.chat_input").await;
        let forward = key(&tree, "ui.root", serde_json::json!({ "key": "Tab" })).await;
        let after = forward["focused"].as_str().unwrap_or("").to_string();
        assert_ne!(after, "ui.chat_input");
        let back = key(
            &tree,
            "ui.root",
            serde_json::json!({ "key": "Tab", "shift": true }),
        )
        .await;
        assert_eq!(back["reverse"], true);
        assert_eq!(back["focused"], "ui.chat_input");
    }

    #[tokio::test]
    async fn alt_tab_cycles_top_level_surfaces() {
        let tree = test_tree();
        seed_shell(&tree).await;
        apply_patch(
            &tree,
            vec![serde_json::json!({
                "op": "insert",
                "anchor": "ui.workspace",
                "node": { "id": "ui.agent_button_1", "type": "button", "props": { "label": "A" } }
            })],
            false,
        )
        .await
        .expect("spawn");
        focus(&tree, "ui.chat_input").await;
        let out = key(
            &tree,
            "ui.root",
            serde_json::json!({ "key": "Tab", "alt": true }),
        )
        .await;
        assert_eq!(out["action"], "surface.cycle");
        assert_eq!(out["focused"], "ui.agent_button_1");
        let back = key(
            &tree,
            "ui.root",
            serde_json::json!({ "key": "Tab", "alt": true, "shift": true }),
        )
        .await;
        assert_eq!(back["reverse"], true);
    }

    #[tokio::test]
    async fn super_key_opens_command_menu_and_escape_closes_it() {
        let tree = test_tree();
        seed_shell(&tree).await;
        let out = key(
            &tree,
            "ui.root",
            serde_json::json!({ "key": "Meta", "meta": true }),
        )
        .await;
        assert_eq!(out["action"], "menu.open");
        {
            let t = tree.lock().await;
            let menu = t.get(keys::MENU_ID).expect("menu node");
            assert_eq!(menu.kind, "list");
        }
        let out = key(&tree, "ui.root", serde_json::json!({ "key": "Escape" })).await;
        assert_eq!(out["action"], "dialog.dismiss");
        let t = tree.lock().await;
        assert!(t.get(keys::MENU_ID).is_none());
    }

    #[tokio::test]
    async fn printscreen_captures_a_tree_snapshot() {
        let tree = test_tree();
        seed_shell(&tree).await;
        let out = key(
            &tree,
            "ui.root",
            serde_json::json!({ "key": "PrintScreen" }),
        )
        .await;
        assert_eq!(out["action"], "snapshot");
        let text = out["snapshot"]["text"].as_str().unwrap_or("");
        assert!(text.contains("ui.chat_input"), "snapshot text: {text}");
    }

    #[tokio::test]
    async fn arrow_keys_navigate_a_focused_list() {
        let tree = test_tree();
        seed_shell(&tree).await;
        apply_patch(
            &tree,
            vec![serde_json::json!({
                "op": "insert",
                "anchor": "ui.workspace",
                "node": {
                    "id": "ui.agent_list_1",
                    "type": "list",
                    "props": { "items": ["a", "b", "c"], "selected": 0, "height": 64 }
                }
            })],
            false,
        )
        .await
        .expect("spawn list");
        focus(&tree, "ui.agent_list_1").await;
        let out = key(
            &tree,
            "ui.agent_list_1",
            serde_json::json!({ "key": "ArrowDown" }),
        )
        .await;
        assert_eq!(out["action"], "list.navigate");
        assert_eq!(out["selected"], 1);
        let out = key(
            &tree,
            "ui.agent_list_1",
            serde_json::json!({ "key": "End" }),
        )
        .await;
        assert_eq!(out["selected"], 2);
        let out = key(
            &tree,
            "ui.agent_list_1",
            serde_json::json!({ "key": "Home" }),
        )
        .await;
        assert_eq!(out["selected"], 0);
        let out = key(
            &tree,
            "ui.agent_list_1",
            serde_json::json!({ "key": "PageDown" }),
        )
        .await;
        assert_eq!(out["action"], "scroll.page_down");
    }

    #[tokio::test]
    async fn ime_dead_key_shows_preedit_then_commits() {
        let tree = test_tree();
        seed_shell(&tree).await;
        focus(&tree, "ui.chat_input").await;
        let pending = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "dead_acute" }),
        )
        .await;
        assert_eq!(pending["action"], "ime.pending");
        {
            let t = tree.lock().await;
            assert!(t
                .get("ui.chat_input")
                .and_then(|n| n.props.get("preedit"))
                .is_some());
        }
        let committed = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "e", "text": "e" }),
        )
        .await;
        assert_eq!(committed["action"], "ime.commit");
        assert_eq!(committed["text"], "é");
        let t = tree.lock().await;
        assert!(t
            .get("ui.chat_input")
            .and_then(|n| n.props.get("preedit"))
            .is_none());
    }

    #[tokio::test]
    async fn escape_cancels_pending_ime_before_dismissing() {
        let tree = test_tree();
        seed_shell(&tree).await;
        focus(&tree, "ui.chat_input").await;
        key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "dead_acute" }),
        )
        .await;
        let out = key(
            &tree,
            "ui.chat_input",
            serde_json::json!({ "key": "Escape" }),
        )
        .await;
        assert_eq!(out["action"], "ime.cancel");
    }

    #[tokio::test]
    // The guard serialises tests against the process-global shortcut table; it is
    // never contended by non-test code, so holding it across awaits is fine.
    #[allow(clippy::await_holding_lock)]
    async fn user_shortcut_replaces_default_chord() {
        let _g = shortcuts::test_guard();
        let tree = test_tree();
        seed_shell(&tree).await;
        shortcuts::reset();
        let resp = handle_request(
            "ui.shortcuts.set".into(),
            Some(serde_json::json!({
                "chord": "ctrl+shift+p",
                "action": "snapshot",
                "description": "custom capture"
            })),
            &tree,
        )
        .await;
        assert!(resp.error.is_none(), "unexpected {:?}", resp.error);
        assert_eq!(resp.result.unwrap()["chord"], "Ctrl+Shift+P");
        let out = key(
            &tree,
            "ui.root",
            serde_json::json!({ "key": "p", "ctrl": true, "shift": true }),
        )
        .await;
        assert_eq!(out["action"], "snapshot");

        let listed = handle_request("ui.shortcuts.list".into(), None, &tree).await;
        let shortcuts_json = listed.result.unwrap();
        assert!(shortcuts_json["shortcuts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["chord"] == "Ctrl+Shift+P" && s["user_defined"] == true));

        handle_request("ui.shortcuts.reset".into(), None, &tree).await;
        let out = key(
            &tree,
            "ui.root",
            serde_json::json!({ "key": "p", "ctrl": true, "shift": true }),
        )
        .await;
        assert_ne!(out["action"], "snapshot");
    }

    #[tokio::test]
    async fn unknown_shortcut_action_is_rejected() {
        let tree = test_tree();
        let resp = handle_request(
            "ui.shortcuts.set".into(),
            Some(serde_json::json!({ "chord": "Ctrl+Shift+Q", "action": "rm -rf" })),
            &tree,
        )
        .await;
        assert_eq!(
            resp.error.as_ref().map(|e| e.code.as_str()),
            Some("E_INVALID")
        );
    }

    #[tokio::test]
    async fn wheel_flick_keeps_gliding_after_the_last_event() {
        let tree = test_tree();
        seed_shell(&tree).await;
        apply_patch(
            &tree,
            vec![serde_json::json!({
                "op": "insert",
                "anchor": "ui.workspace",
                "node": {
                    "id": "ui.agent_list_1",
                    "type": "list",
                    "props": { "items": (0..80).map(|i| format!("row {i}")).collect::<Vec<_>>(), "height": 160 }
                }
            })],
            false,
        )
        .await
        .expect("spawn list");
        focus(&tree, "ui.agent_list_1").await;
        let resp = handle_request(
            "ui.event".into(),
            Some(serde_json::json!({
                "id": "ui.agent_list_1",
                "event": "wheel",
                "payload": { "delta_y": 90 }
            })),
            &tree,
        )
        .await;
        let at_release = resp.result.unwrap()["scroll_y"].as_i64().unwrap();
        assert_eq!(at_release, 90);
        // The glide task decays momentum on its own; offset keeps growing.
        for _ in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            let t = tree.lock().await;
            let v = t.get("ui.agent_list_1").unwrap().props["scroll_velocity"]
                .as_f64()
                .unwrap();
            if v == 0.0 {
                break;
            }
        }
        let t = tree.lock().await;
        let node = t.get("ui.agent_list_1").unwrap();
        assert_eq!(node.props["scroll_velocity"].as_f64(), Some(0.0));
        assert!(
            node.props["scroll_y"].as_i64().unwrap() > at_release,
            "flick should coast past the release offset"
        );
    }

    #[tokio::test]
    async fn ui_scroll_moves_a_list_programmatically() {
        let tree = test_tree();
        seed_shell(&tree).await;
        apply_patch(
            &tree,
            vec![serde_json::json!({
                "op": "insert",
                "anchor": "ui.workspace",
                "node": {
                    "id": "ui.agent_list_1",
                    "type": "list",
                    "props": { "items": (0..40).map(|i| format!("row {i}")).collect::<Vec<_>>(), "height": 160 }
                }
            })],
            false,
        )
        .await
        .expect("spawn list");
        let resp = handle_request(
            "ui.scroll".into(),
            Some(serde_json::json!({ "id": "ui.agent_list_1", "pages": 1 })),
            &tree,
        )
        .await;
        assert_eq!(resp.result.unwrap()["scroll_y"], 160);
        let resp = handle_request(
            "ui.scroll".into(),
            Some(serde_json::json!({ "id": "ui.agent_list_1" })),
            &tree,
        )
        .await;
        assert_eq!(
            resp.error.as_ref().map(|e| e.code.as_str()),
            Some("E_INVALID")
        );
    }

    #[tokio::test]
    async fn a11y_announce_records_live_region_and_focus_order() {
        let tree = test_tree();
        seed_shell(&tree).await;
        let resp = handle_request(
            "ui.a11y.announce".into(),
            Some(serde_json::json!({ "message": "Workspace cleared", "live": "assertive" })),
            &tree,
        )
        .await;
        assert!(resp.error.is_none());
        let result = resp.result.unwrap();
        assert_eq!(result["announced"], "Workspace cleared");
        {
            let t = tree.lock().await;
            assert_eq!(
                t.get("ui.activity")
                    .and_then(|n| n.props.get("text"))
                    .and_then(|v| v.as_str()),
                Some("Workspace cleared")
            );
            assert_eq!(t.announcement.as_deref(), Some("Workspace cleared"));
        }
        let order = handle_request("ui.a11y.focus_order".into(), None, &tree).await;
        let order = order.result.unwrap();
        let ids: Vec<&str> = order["order"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["id"].as_str())
            .collect();
        assert!(ids.contains(&"ui.chat_input"));
        assert!(ids.contains(&"ui.chat_send"));
        let roles: Vec<&str> = order["order"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["role"].as_str())
            .collect();
        assert!(roles.contains(&"textfield"));
        assert!(roles.contains(&"button"));
    }

    #[tokio::test]
    async fn dialog_traps_focus_order() {
        let tree = test_tree();
        seed_shell(&tree).await;
        apply_patch(
            &tree,
            vec![
                serde_json::json!({
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": { "id": "ui.agent_dialog_1", "type": "dialog", "props": { "label": "D" } }
                }),
                serde_json::json!({
                    "op": "insert",
                    "anchor": "ui.agent_dialog_1",
                    "node": { "id": "ui.agent_dialog_1_ok", "type": "button", "props": { "label": "OK" } }
                }),
            ],
            false,
        )
        .await
        .expect("dialog");
        let order = handle_request("ui.a11y.focus_order".into(), None, &tree).await;
        let order = order.result.unwrap();
        assert_eq!(order["trapped"], true);
        let ids: Vec<&str> = order["order"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["id"].as_str())
            .collect();
        assert!(ids.contains(&"ui.agent_dialog_1_ok"));
        assert!(
            !ids.contains(&"ui.chat_input"),
            "focus must not escape the dialog: {ids:?}"
        );
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
