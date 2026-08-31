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
mod layout;
mod motion;
mod renderer;
mod scroll;
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
    name: String,
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
            name: "dark".into(),
            colors,
            spacing,
            rounding,
            typography,
        }
    }

    fn named(name: &str) -> Self {
        match name {
            "light" => Self::from_asl(asl::design_system_light_asl(), "light"),
            "high-contrast" | "high_contrast" | "hc" => {
                Self::from_asl(asl::design_system_high_contrast_asl(), "high-contrast")
            }
            _ => Self::design_system_dark(),
        }
    }

    fn from_asl(src: &str, name: &str) -> Self {
        let mut t = Self::from_asl_inner(src);
        t.name = name.to_string();
        t
    }

    fn from_asl_inner(src: &str) -> Self {
        let doc = asl::parse_asl(src);
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
        ] {
            typography.insert(k.into(), serde_json::json!(v));
        }
        typography.insert("family.default".into(), serde_json::json!("Inter"));
        typography.insert("family.numeric".into(), serde_json::json!("JetBrains Mono"));
        Theme {
            name: "dark".into(),
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
    drag: Option<dnd::DragSession>,
    /// Dead-key / compose IME state for the focused field.
    ime: ime::ImeState,
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
        "ui.workspace.clear" => {
            let preserve_hint = params
                .and_then(|p| p.get("preserve_hint").and_then(|v| v.as_bool()))
                .unwrap_or(true);
            match clear_workspace(tree, preserve_hint, true).await {
                Ok(removed) => {
                    success_response(&id, serde_json::json!({ "ok": true, "removed": removed }))
                }
                Err(e) => error_response(&id, "E_CLEAR_FAILED", &e),
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
                        if node.kind == "chart" {
                            let data = node
                                .props
                                .get("data")
                                .or_else(|| node.props.get("items"))
                                .cloned()
                                .unwrap_or(serde_json::Value::Null);
                            let idx = event_payload
                                .get("index")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as usize;
                            let tip = data
                                .as_array()
                                .and_then(|a| a.get(idx))
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| data.to_string());
                            node.props.insert("tooltip".into(), serde_json::json!(tip));
                        }
                        if node.kind == "media" {
                            let playing = node
                                .props
                                .get("playing")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            node.props
                                .insert("playing".into(), serde_json::json!(!playing));
                        }
                        if node.kind == "list" {
                            if let Some(idx) = event_payload.get("index").and_then(|v| v.as_u64()) {
                                node.props
                                    .insert("selected_index".into(), serde_json::json!(idx));
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

            if event == "context"
                || event_payload
                    .get("button")
                    .and_then(|v| v.as_u64())
                    .is_some_and(|b| b == 3)
            {
                let snapshot = {
                    let mut t = tree.lock().await;
                    let label = t
                        .get(&nid)
                        .map(|n| {
                            n.props
                                .get("label")
                                .or_else(|| n.props.get("text"))
                                .and_then(|v| v.as_str())
                                .unwrap_or(&n.id)
                                .to_string()
                        })
                        .unwrap_or_else(|| nid.clone());
                    let dialog = UiNode {
                        id: "ui.context_menu".into(),
                        kind: "dialog".into(),
                        props: {
                            let mut m = HashMap::new();
                            m.insert("label".into(), serde_json::json!("Actions"));
                            m.insert("text".into(), serde_json::json!(format!("For {label}")));
                            m.insert("dismissible".into(), serde_json::json!(true));
                            m
                        },
                        children: vec![],
                        asl_style: None,
                        bindings: vec![],
                    };
                    t.nodes.insert("ui.context_menu".into(), dialog);
                    let root_id = t.root_id.clone();
                    if let Some(root) = t.get_mut(&root_id) {
                        if !root.children.contains(&"ui.context_menu".to_string()) {
                            root.children.push("ui.context_menu".into());
                        }
                    }
                    t.revision += 1;
                    renderer::serialize_subtree(&t, t.root_id())
                };
                let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                return success_response(
                    &id,
                    serde_json::json!({ "handled": 1, "action": "context-menu", "id": nid }),
                );
            }

            // Wheel → scroll focused list / overflow container.
            if event == "wheel" {
                let dy = event_payload
                    .get("delta_y")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;
                let snapshot = {
                    let mut t = tree.lock().await;
                    let target = t
                        .focused()
                        .map(|s| s.to_string())
                        .filter(|id| t.get(id).is_some_and(|n| n.kind == "list"))
                        .unwrap_or_else(|| nid.clone());
                    if let Some(node) = t.get_mut(&target) {
                        if node.kind == "list" {
                            let vh = node
                                .props
                                .get("height")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(160) as u32;
                            scroll::apply_wheel_kinetic(&mut node.props, dy, vh);
                            t.revision += 1;
                        }
                    }
                    renderer::serialize_subtree(&t, t.root_id())
                };
                let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                return success_response(
                    &id,
                    serde_json::json!({ "handled": 1, "action": "scroll" }),
                );
            }

            // Keyboard editing on the focused field.
            if event == "key" {
                let key = event_payload
                    .get("key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let text = event_payload.get("text").and_then(|v| v.as_str());
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
                    let next = {
                        let mut t = tree.lock().await;
                        let next = focus::next_focus(&t, t.focused(), mods.shift);
                        t.set_focused(next.clone());
                        next
                    };
                    if let Some(ref sid) = next {
                        let _ = mcp_call(
                            "compositor.focus",
                            serde_json::json!({ "id": format!("surface.{sid}") }),
                        )
                        .await;
                    }
                    return success_response(
                        &id,
                        serde_json::json!({ "handled": 1, "focused": next, "action": "tab" }),
                    );
                }

                // Desktop chords → Machine-native AUIL/MCP actions (not OS window manager).
                if (mods.alt && key == "Tab")
                    || key == "Super_L"
                    || key == "Super"
                    || key == "Meta_L"
                {
                    let next = {
                        let mut t = tree.lock().await;
                        let next = focus::next_focus(&t, t.focused(), mods.shift);
                        t.set_focused(next.clone());
                        next
                    };
                    return success_response(
                        &id,
                        serde_json::json!({ "handled": 1, "action": "workspace.cycle", "focused": next }),
                    );
                }
                if key == "F1" {
                    let snapshot = {
                        let mut t = tree.lock().await;
                        let node = UiNode {
                            id: "ui.help_dialog".into(),
                            kind: "dialog".into(),
                            props: {
                                let mut m = HashMap::new();
                                m.insert("label".into(), serde_json::json!("Help"));
                                m.insert(
                                    "text".into(),
                                    serde_json::json!(
                                        "Enter sends chat. Tab moves focus. Ask to place controls."
                                    ),
                                );
                                m.insert("dismissible".into(), serde_json::json!(true));
                                m
                            },
                            children: vec![],
                            asl_style: None,
                            bindings: vec![],
                        };
                        t.nodes.insert("ui.help_dialog".into(), node);
                        let root = t.root_id.clone();
                        if let Some(p) = t.get_mut(&root) {
                            if !p.children.contains(&"ui.help_dialog".to_string()) {
                                p.children.push("ui.help_dialog".into());
                            }
                        }
                        t.revision += 1;
                        renderer::serialize_subtree(&t, t.root_id())
                    };
                    let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                    return success_response(
                        &id,
                        serde_json::json!({ "handled": 1, "action": "help" }),
                    );
                }
                if key == "F5" {
                    let _ = mcp_call("agent.status", serde_json::json!({})).await;
                    return success_response(
                        &id,
                        serde_json::json!({ "handled": 1, "action": "refresh" }),
                    );
                }
                if key == "F12" {
                    let _ = mcp_call("ui.status", serde_json::json!({})).await;
                    return success_response(
                        &id,
                        serde_json::json!({ "handled": 1, "action": "debug.status" }),
                    );
                }
                if key == "Print" || key == "PrintScreen" {
                    let _ = mcp_call(
                        "clipboard.set",
                        serde_json::json!({ "text": "The Machine · session screenshot token" }),
                    )
                    .await;
                    return success_response(
                        &id,
                        serde_json::json!({ "handled": 1, "action": "clipboard.capture" }),
                    );
                }
                if mods.ctrl && mods.shift && key.to_ascii_lowercase() == "t" {
                    let _ = mcp_call(
                        "agent.chat.send",
                        serde_json::json!({ "text": "show suggestions", "source": "chat_ui" }),
                    )
                    .await;
                    return success_response(
                        &id,
                        serde_json::json!({ "handled": 1, "action": "chat.suggestions" }),
                    );
                }
                if mods.ctrl && mods.alt && (key == "F9" || key == "f9") {
                    let _ = mcp_call("hello", serde_json::json!({})).await;
                    return success_response(
                        &id,
                        serde_json::json!({ "handled": 1, "action": "fallback.hello" }),
                    );
                }
                if matches!(key, "PageUp" | "PageDown") {
                    let handled = {
                        let mut t = tree.lock().await;
                        let target = t
                            .focused()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| nid.clone());
                        let is_list = t.get(&target).is_some_and(|n| n.kind == "list");
                        if is_list {
                            if let Some(node) = t.get_mut(&target) {
                                let vh = node
                                    .props
                                    .get("height")
                                    .and_then(|v| v.as_u64())
                                    .unwrap_or(160) as u32;
                                let dir = if key == "PageUp" { -1 } else { 1 };
                                let mut state = scroll::ScrollState::from_props(
                                    &serde_json::to_value(&node.props).unwrap_or_default(),
                                    vh,
                                    node.props
                                        .get("content_h")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(400) as u32,
                                );
                                state.scroll_page(dir);
                                node.props
                                    .insert("scroll_y".into(), serde_json::json!(state.offset_y));
                                t.revision += 1;
                            }
                            let snapshot = renderer::serialize_subtree(&t, t.root_id());
                            Some(snapshot)
                        } else {
                            None
                        }
                    };
                    if let Some(snapshot) = handled {
                        let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                        return success_response(
                            &id,
                            serde_json::json!({ "handled": 1, "action": "page-scroll" }),
                        );
                    }
                }
                if matches!(key, "ArrowUp" | "ArrowDown" | "Up" | "Down") {
                    let handled = {
                        let mut t = tree.lock().await;
                        let focus_id = t.focused().unwrap_or(&nid).to_string();
                        let is_list = t.get(&focus_id).is_some_and(|n| n.kind == "list");
                        if is_list {
                            if let Some(node) = t.get_mut(&focus_id) {
                                let len = node
                                    .props
                                    .get("items")
                                    .and_then(|v| v.as_array())
                                    .map(|a| a.len())
                                    .unwrap_or(1)
                                    .max(1);
                                let cur =
                                    node.props
                                        .get("selected_index")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(0) as usize;
                                let next = if key.contains("Up") {
                                    cur.saturating_sub(1)
                                } else {
                                    (cur + 1).min(len - 1)
                                };
                                node.props
                                    .insert("selected_index".into(), serde_json::json!(next));
                                t.revision += 1;
                            }
                            let snapshot = renderer::serialize_subtree(&t, t.root_id());
                            Some(snapshot)
                        } else {
                            None
                        }
                    };
                    if let Some(snapshot) = handled {
                        let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                        return success_response(
                            &id,
                            serde_json::json!({ "handled": 1, "action": "list.navigate" }),
                        );
                    }
                }

                // Escape: cancel pending IME first, else dismiss soft dialog.
                if key == "Escape" {
                    let cancelled_ime = {
                        let mut t = tree.lock().await;
                        if t.ime.pending.is_some() {
                            t.ime.reset();
                            true
                        } else {
                            false
                        }
                    };
                    if cancelled_ime {
                        return success_response(
                            &id,
                            serde_json::json!({
                                "handled": 1,
                                "action": "ime.cancel",
                            }),
                        );
                    }
                    let dismissed = {
                        let mut t = tree.lock().await;
                        let dialog_id = t
                            .nodes
                            .iter()
                            .find(|(_, n)| n.kind == "dialog")
                            .map(|(id, _)| id.clone());
                        if let Some(did) = dialog_id.clone() {
                            t.detach(&did);
                            remove_subtree(&mut t, &did);
                            t.revision += 1;
                            let snapshot = renderer::serialize_subtree(&t, t.root_id());
                            Some((did, snapshot))
                        } else {
                            None
                        }
                    };
                    if let Some((did, snapshot)) = dismissed {
                        let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                        let _ = mcp_call(
                            "compositor.surface",
                            serde_json::json!({
                                "action": "destroy",
                                "id": format!("surface.{did}"),
                            }),
                        )
                        .await;
                        return success_response(
                            &id,
                            serde_json::json!({
                                "handled": 1,
                                "action": "dialog.dismiss",
                                "id": did,
                            }),
                        );
                    }
                }

                // Enter in chat field → send (same as clicking Send).
                if matches!(key, "Enter" | "Return") {
                    let focus_id = {
                        let t = tree.lock().await;
                        t.focused().unwrap_or(&nid).to_string()
                    };
                    if focus_id == "ui.chat_input" {
                        if let Some(result) = send_chat_from_input(tree).await {
                            return success_response(
                                &id,
                                serde_json::json!({
                                    "handled": 1,
                                    "action": "chat.send",
                                    "result": result,
                                }),
                            );
                        }
                    }
                }

                // Enter / Return activates the focused button (same as press).
                if matches!(key, "Enter" | "Return") {
                    let focus_id = {
                        let t = tree.lock().await;
                        t.focused().unwrap_or(&nid).to_string()
                    };
                    let is_button = {
                        let t = tree.lock().await;
                        t.get(&focus_id).is_some_and(|n| n.kind == "button")
                    };
                    if is_button {
                        nid = focus_id;
                        // Fall through to binding execution as a press.
                        let snapshot = {
                            let mut t = tree.lock().await;
                            if let Some(node) = t.get_mut(&nid) {
                                node.props.insert("pressed".into(), serde_json::json!(true));
                            }
                            renderer::serialize_subtree(&t, t.root_id())
                        };
                        let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                        // Reuse press binding path below by rewriting event.
                        let event = "press".to_string();
                        let bindings = {
                            let t = tree.lock().await;
                            let node = t.get(&nid);
                            let props = node
                                .map(|n| {
                                    serde_json::to_value(&n.props)
                                        .unwrap_or(serde_json::Value::Null)
                                })
                                .unwrap_or(serde_json::Value::Null);
                            let b = node.map(|n| n.bindings.clone()).unwrap_or_default();
                            let chat_text = t.get("ui.chat_input").and_then(|n| {
                                n.props
                                    .get("text")
                                    .or_else(|| n.props.get("value"))
                                    .and_then(|v| v.as_str())
                                    .filter(|s| !s.is_empty())
                                    .map(|s| s.to_string())
                            });
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
                            if b.target == "agent.chat.send" && r.is_some() {
                                let _ = clear_chat_input(tree).await;
                            }
                            results.push(serde_json::json!({ "target": b.target, "result": r }));
                        }
                        // Clear press feedback.
                        let snapshot = {
                            let mut t = tree.lock().await;
                            if let Some(node) = t.get_mut(&nid) {
                                node.props
                                    .insert("pressed".into(), serde_json::json!(false));
                            }
                            renderer::serialize_subtree(&t, t.root_id())
                        };
                        let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                        return success_response(
                            &id,
                            serde_json::json!({
                                "handled": results.len(),
                                "results": results,
                                "action": "activate",
                            }),
                        );
                    }
                }

                // Ctrl/Cmd-C/V/X clipboard on focused field.
                if mods.ctrl || mods.meta {
                    let focus_id = {
                        let t = tree.lock().await;
                        t.focused().unwrap_or(&nid).to_string()
                    };
                    let key_l = key.to_ascii_lowercase();
                    if matches!(key_l.as_str(), "c" | "v" | "x" | "a" | "z") {
                        let mut t = tree.lock().await;
                        let edited = if let Some(node) = t.get(&focus_id) {
                            if matches!(node.kind.as_str(), "field" | "input") {
                                let current = node
                                    .props
                                    .get("text")
                                    .or_else(|| node.props.get("value"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let caret = node
                                    .props
                                    .get("caret")
                                    .and_then(|v| v.as_u64())
                                    .map(|n| n as usize)
                                    .unwrap_or(current.len());
                                let sel = node
                                    .props
                                    .get("sel_anchor")
                                    .and_then(|v| v.as_u64())
                                    .map(|n| n as usize);
                                let clip = if let Some(a) = sel {
                                    if a != caret {
                                        let (s, e) = (a.min(caret), a.max(caret));
                                        current.get(s..e).unwrap_or(&current).to_string()
                                    } else {
                                        current.clone()
                                    }
                                } else {
                                    current.clone()
                                };
                                Some((current, clip, caret, sel))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        if let Some((current, clip, caret, sel)) = edited {
                            match key_l.as_str() {
                                "a" => {
                                    if let Some(node) = t.get_mut(&focus_id) {
                                        node.props
                                            .insert("sel_anchor".into(), serde_json::json!(0));
                                        node.props.insert(
                                            "sel_end".into(),
                                            serde_json::json!(current.len()),
                                        );
                                        node.props.insert(
                                            "caret".into(),
                                            serde_json::json!(current.len()),
                                        );
                                    }
                                    t.revision += 1;
                                    let snapshot = renderer::serialize_subtree(&t, t.root_id());
                                    drop(t);
                                    let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                                    return success_response(
                                        &id,
                                        serde_json::json!({
                                            "handled": 1,
                                            "action": "select.all",
                                        }),
                                    );
                                }
                                "z" => {
                                    if let Some(node) = t.get_mut(&focus_id) {
                                        let prev = node
                                            .props
                                            .get("last_text")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        node.props
                                            .insert("last_text".into(), serde_json::json!(current));
                                        node.props
                                            .insert("text".into(), serde_json::json!(prev.clone()));
                                        node.props.insert(
                                            "value".into(),
                                            serde_json::json!(prev.clone()),
                                        );
                                        node.props
                                            .insert("caret".into(), serde_json::json!(prev.len()));
                                    }
                                    t.revision += 1;
                                    let snapshot = renderer::serialize_subtree(&t, t.root_id());
                                    drop(t);
                                    let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                                    return success_response(
                                        &id,
                                        serde_json::json!({
                                            "handled": 1,
                                            "action": "edit.undo",
                                        }),
                                    );
                                }
                                "c" => {
                                    drop(t);
                                    let _ = mcp_call(
                                        "clipboard.set",
                                        serde_json::json!({ "text": clip }),
                                    )
                                    .await;
                                    return success_response(
                                        &id,
                                        serde_json::json!({
                                            "handled": 1,
                                            "action": "clipboard.copy",
                                        }),
                                    );
                                }
                                "x" => {
                                    let remaining = if let Some(a) = sel {
                                        if a != caret {
                                            let (s, e) = (a.min(caret), a.max(caret));
                                            format!(
                                                "{}{}",
                                                current.get(..s).unwrap_or(""),
                                                current.get(e..).unwrap_or("")
                                            )
                                        } else {
                                            String::new()
                                        }
                                    } else {
                                        String::new()
                                    };
                                    if let Some(node) = t.get_mut(&focus_id) {
                                        node.props.insert(
                                            "text".into(),
                                            serde_json::json!(remaining.clone()),
                                        );
                                        node.props.insert(
                                            "value".into(),
                                            serde_json::json!(remaining.clone()),
                                        );
                                        node.props.insert(
                                            "caret".into(),
                                            serde_json::json!(sel
                                                .unwrap_or(0)
                                                .min(remaining.len())),
                                        );
                                        node.props.remove("sel_anchor");
                                    }
                                    t.revision += 1;
                                    let snapshot = renderer::serialize_subtree(&t, t.root_id());
                                    drop(t);
                                    let _ = mcp_call(
                                        "clipboard.set",
                                        serde_json::json!({ "text": clip }),
                                    )
                                    .await;
                                    let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                                    return success_response(
                                        &id,
                                        serde_json::json!({
                                            "handled": 1,
                                            "action": "clipboard.cut",
                                        }),
                                    );
                                }
                                "v" => {
                                    drop(t);
                                    let clip = mcp_call("clipboard.get", serde_json::json!({}))
                                        .await
                                        .and_then(|v| {
                                            v.get("text")
                                                .and_then(|t| t.as_str())
                                                .map(|s| s.to_string())
                                        })
                                        .unwrap_or_default();
                                    let mut t = tree.lock().await;
                                    if let Some(node) = t.get_mut(&focus_id) {
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
                                        let sel_anchor = node
                                            .props
                                            .get("sel_anchor")
                                            .and_then(|v| v.as_u64())
                                            .map(|n| n as usize);
                                        let mut buf = input_edit::TextBuffer::from_props_sel(
                                            current, caret, sel_anchor,
                                        );
                                        buf.insert_str(&clip);
                                        node.props.insert(
                                            "text".into(),
                                            serde_json::json!(buf.text.clone()),
                                        );
                                        node.props.insert(
                                            "value".into(),
                                            serde_json::json!(buf.text.clone()),
                                        );
                                        node.props
                                            .insert("caret".into(), serde_json::json!(buf.caret));
                                        node.props.remove("sel_anchor");
                                    }
                                    t.revision += 1;
                                    let snapshot = renderer::serialize_subtree(&t, t.root_id());
                                    drop(t);
                                    let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                                    return success_response(
                                        &id,
                                        serde_json::json!({
                                            "handled": 1,
                                            "action": "clipboard.paste",
                                        }),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Dead-key / compose IME before ordinary field editing.
                if !(mods.ctrl || mods.meta) {
                    let ime_result = {
                        let mut t = tree.lock().await;
                        let out = t.ime.feed(key, text);
                        match out {
                            ime::ImeOutput::Pending => Some(("pending".to_string(), None)),
                            ime::ImeOutput::Commit(composed) => {
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
                                        let mut buf =
                                            input_edit::TextBuffer::from_props(current, caret);
                                        buf.insert_str(&composed);
                                        if let Some(node) = t.get_mut(&focus_id) {
                                            node.props.insert(
                                                "text".into(),
                                                serde_json::json!(buf.text.clone()),
                                            );
                                            node.props.insert(
                                                "value".into(),
                                                serde_json::json!(buf.text.clone()),
                                            );
                                            node.props.insert(
                                                "caret".into(),
                                                serde_json::json!(buf.caret),
                                            );
                                        }
                                        t.revision += 1;
                                        let snapshot = renderer::serialize_subtree(&t, t.root_id());
                                        Some((composed, snapshot))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                };
                                Some(("commit".to_string(), edited))
                            }
                            ime::ImeOutput::Pass => None,
                        }
                    };
                    if let Some((action, edited)) = ime_result {
                        if action == "pending" {
                            return success_response(
                                &id,
                                serde_json::json!({
                                    "handled": 1,
                                    "action": "ime.pending",
                                }),
                            );
                        }
                        if let Some((composed, snapshot)) = edited {
                            let _ = renderer::sync_tree_to_compositor(&snapshot).await;
                            return success_response(
                                &id,
                                serde_json::json!({
                                    "handled": 1,
                                    "action": "ime.commit",
                                    "text": composed,
                                }),
                            );
                        }
                    }
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
                            .unwrap_or("")
                            .to_string();
                        let caret = node
                            .props
                            .get("caret")
                            .and_then(|v| v.as_u64())
                            .map(|n| n as usize);
                        let mut buf = input_edit::TextBuffer::from_props(&current, caret);
                        if input_edit::apply_key(&mut buf, key, text, &mods) {
                            if let Some(node) = t.get_mut(&focus_id) {
                                node.props
                                    .insert("last_text".into(), serde_json::json!(current));
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
                if b.target == "agent.chat.send" && r.is_some() {
                    let _ = clear_chat_input(tree).await;
                }
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
        "ui.theme.set" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                let mut t = tree.lock().await;
                t.theme = Theme::named(name);
                return success_response(
                    &id,
                    serde_json::json!({ "ok": true, "name": t.theme.name }),
                );
            }
            let theme = params
                .get("theme")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let mut t = tree.lock().await;
            if let Ok(th) = serde_json::from_value::<Theme>(theme) {
                t.theme = th;
                success_response(&id, serde_json::json!({"ok": true, "name": t.theme.name}))
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
    let mut node: UiNode =
        serde_json::from_value(value.clone()).map_err(|e| format!("invalid node: {}", e))?;
    // Refuse empty accessible names: fall back to id so AT and HIG stay honest.
    if matches!(node.kind.as_str(), "button" | "toggle" | "slider") {
        let empty = node
            .props
            .get("label")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().is_empty())
            .unwrap_or(true);
        if empty {
            node.props
                .insert("label".into(), serde_json::json!(node.id.clone()));
            node.props
                .insert("empty_label_refused".into(), serde_json::json!(true));
        }
    }
    Ok(node)
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
                // Duplicate id: last writer wins (replace in place).
                if t.nodes.contains_key(&nid) {
                    t.detach(&nid);
                    remove_subtree(&mut t, &nid);
                }
                t.nodes.insert(nid.clone(), node);
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

async fn clear_workspace(
    tree: &SharedTree,
    preserve_hint: bool,
    sync_compositor: bool,
) -> Result<Vec<String>, String> {
    let (removed, _rev, snapshot) = {
        let mut t = tree.lock().await;
        let workspace_id = "ui.workspace".to_string();
        let children: Vec<String> = t
            .get(&workspace_id)
            .map(|n| n.children.clone())
            .unwrap_or_default();
        let mut removed = Vec::new();
        for child in children {
            if preserve_hint && child == "ui.workspace_hint" {
                continue;
            }
            t.detach(&child);
            remove_subtree(&mut t, &child);
            removed.push(child);
        }
        t.revision += 1;
        let rev = t.revision;
        let snapshot = renderer::serialize_subtree(&t, t.root_id());
        (removed, rev, snapshot)
    };
    if sync_compositor {
        let _ = renderer::sync_tree_to_compositor(&snapshot).await;
    }
    Ok(removed)
}

async fn clear_chat_input(tree: &SharedTree) -> Result<(), String> {
    let snapshot = {
        let mut t = tree.lock().await;
        if let Some(node) = t.get_mut("ui.chat_input") {
            node.props.insert("text".into(), serde_json::json!(""));
            node.props.insert("value".into(), serde_json::json!(""));
            node.props.insert("caret".into(), serde_json::json!(0));
        }
        t.revision += 1;
        renderer::serialize_subtree(&t, t.root_id())
    };
    let _ = renderer::sync_tree_to_compositor(&snapshot).await;
    Ok(())
}

async fn send_chat_from_input(tree: &SharedTree) -> Option<serde_json::Value> {
    let text = {
        let t = tree.lock().await;
        t.get("ui.chat_input").and_then(|n| {
            n.props
                .get("text")
                .or_else(|| n.props.get("value"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
    }?;
    if text.trim().is_empty() {
        return None;
    }
    let result = mcp_call(
        "agent.chat.send",
        serde_json::json!({ "text": text, "source": "chat_ui" }),
    )
    .await;
    if result.is_some() {
        let _ = clear_chat_input(tree).await;
    }
    result
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

    #[tokio::test]
    async fn workspace_clear_preserves_hint() {
        let tree = test_tree();
        let _ = handle_request(
            "ui.patch".into(),
            Some(serde_json::json!({
                "ops": [
                    {
                        "op": "insert",
                        "anchor": "ui.root",
                        "node": { "id": "ui.workspace", "type": "stack", "children": ["ui.workspace_hint", "ui.agent_button"] }
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "node": { "id": "ui.workspace_hint", "type": "text", "props": { "text": "hint" } }
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "node": { "id": "ui.agent_button", "type": "button", "props": { "label": "Go" } }
                    }
                ]
            })),
            &tree,
        )
        .await;
        let resp = handle_request(
            "ui.workspace.clear".into(),
            Some(serde_json::json!({ "preserve_hint": true })),
            &tree,
        )
        .await;
        assert!(resp.error.is_none(), "{:?}", resp.error);
        let removed = resp
            .result
            .as_ref()
            .and_then(|v| v.get("removed"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        assert!(removed
            .iter()
            .any(|v| v.as_str() == Some("ui.agent_button")));
        assert!(!removed
            .iter()
            .any(|v| v.as_str() == Some("ui.workspace_hint")));
    }

    #[tokio::test]
    async fn insert_replaces_duplicate_id() {
        let tree = test_tree();
        let _ = handle_request(
            "ui.patch".into(),
            Some(serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.root",
                    "node": { "id": "ui.agent_button", "type": "button", "props": { "label": "One" } }
                }]
            })),
            &tree,
        )
        .await;
        let _ = handle_request(
            "ui.patch".into(),
            Some(serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.root",
                    "node": { "id": "ui.agent_button", "type": "button", "props": { "label": "Two" } }
                }]
            })),
            &tree,
        )
        .await;
        let got = handle_request(
            "ui.get".into(),
            Some(serde_json::json!({ "id": "ui.agent_button" })),
            &tree,
        )
        .await;
        assert_eq!(
            got.result
                .as_ref()
                .and_then(|v| v.get("props"))
                .and_then(|p| p.get("label"))
                .and_then(|v| v.as_str()),
            Some("Two")
        );
    }

    #[tokio::test]
    async fn refuse_empty_button_label() {
        let tree = test_tree();
        let _ = handle_request(
            "ui.patch".into(),
            Some(serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.root",
                    "node": { "id": "ui.blank", "type": "button", "props": { "label": "" } }
                }]
            })),
            &tree,
        )
        .await;
        let got = handle_request(
            "ui.get".into(),
            Some(serde_json::json!({ "id": "ui.blank" })),
            &tree,
        )
        .await;
        let label = got
            .result
            .as_ref()
            .and_then(|v| v.get("props"))
            .and_then(|p| p.get("label"))
            .and_then(|v| v.as_str());
        assert_eq!(label, Some("ui.blank"));
    }

    #[tokio::test]
    async fn theme_named_light_and_high_contrast() {
        let tree = test_tree();
        let resp = handle_request(
            "ui.theme.set".into(),
            Some(serde_json::json!({ "name": "light" })),
            &tree,
        )
        .await;
        assert!(resp.error.is_none());
        let get = handle_request("ui.theme.get".into(), None, &tree).await;
        assert_eq!(
            get.result
                .as_ref()
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str()),
            Some("light")
        );
        let _ = handle_request(
            "ui.theme.set".into(),
            Some(serde_json::json!({ "name": "high-contrast" })),
            &tree,
        )
        .await;
        let get = handle_request("ui.theme.get".into(), None, &tree).await;
        assert_eq!(
            get.result
                .as_ref()
                .and_then(|v| v.get("name"))
                .and_then(|v| v.as_str()),
            Some("high-contrast")
        );
    }

    #[test]
    fn parse_node_refuses_empty_toggle_label() {
        let n = parse_node(&serde_json::json!({
            "id": "t1",
            "type": "toggle",
            "props": {}
        }))
        .unwrap();
        assert_eq!(n.props.get("label").and_then(|v| v.as_str()), Some("t1"));
    }
}
