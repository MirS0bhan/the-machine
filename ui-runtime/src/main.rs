//! UI Runtime - declarative renderer for the UI State Tree (AUIL) with ASL styling.
//!
//! Maintains an in-memory UI tree, applies incremental patch operations
//! (update / insert / remove / replace / move), resolves ASL style tokens
//! against the active theme, and reflects changes to the State Store.

use common::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufReader};
use tokio::sync::Mutex;
use tracing::{info, warn, error};

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

struct UiTree {
    nodes: HashMap<String, UiNode>,
    root_id: String,
    theme: Theme,
    mixins: HashMap<String, serde_json::Value>,
    revision: u64,
    dirty: HashSet<String>,
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
            theme: Theme::default(),
            mixins: HashMap::new(),
            revision: 1,
            dirty: HashSet::new(),
        }
    }

    fn get(&self, id: &str) -> Option<&UiNode> {
        self.nodes.get(id)
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

    let socket_path = "/run/the-machine/ui-runtime.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;
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

async fn handle_request(method: String, params: Option<serde_json::Value>, tree: &SharedTree) -> McpMessage {
    let id = Uuid::new_v4();
    match method.as_str() {
        "ui.patch" => {
            let ops = params
                .and_then(|p| p.get("ops").cloned())
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();
            match apply_patch(tree, ops).await {
                Ok(rev) => success_response(&id, serde_json::json!({ "revision": rev })),
                Err(e) => error_response(&id, "E_PATCH_FAILED", &e),
            }
        }
        "ui.get" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let idp = params.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let t = tree.lock().await;
            if let Some(nid) = idp {
                match t.get(&nid) {
                    Some(node) => success_response(&id, serde_json::to_value(node).unwrap_or(serde_json::Value::Null)),
                    None => error_response(&id, "E_NOT_FOUND", "node not found"),
                }
            } else {
                let root = t.get(&t.root_id).cloned();
                success_response(&id, serde_json::to_value(root).unwrap_or(serde_json::Value::Null))
            }
        }
        "ui.tree" => {
            let t = tree.lock().await;
            let root = t.get(&t.root_id).cloned();
            success_response(&id, serde_json::to_value(root).unwrap_or(serde_json::Value::Null))
        }
        "ui.bind" => {
            // Register a binding on a node (state:* two-way or mcp: one-way).
            let params = params.unwrap_or(serde_json::Value::Null);
            let nid = params.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
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
        "ui.theme.set" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let theme = params.get("theme").cloned().unwrap_or(serde_json::Value::Null);
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
            success_response(&id, serde_json::to_value(&t.theme).unwrap_or(serde_json::Value::Null))
        }
        "ui.status" => {
            let t = tree.lock().await;
            success_response(&id, serde_json::json!({
                "status": "running",
                "revision": t.revision,
                "nodes": t.nodes.len(),
            }))
        }
        _ => error_response(&id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
    }
}

fn parse_node(value: &serde_json::Value) -> Result<UiNode, String> {
    serde_json::from_value(value.clone()).map_err(|e| format!("invalid node: {}", e))
}

async fn apply_patch(tree: &SharedTree, ops: Vec<serde_json::Value>) -> Result<u64, String> {
    let mut t = tree.lock().await;
    for op in &ops {
        let kind = op
            .get("op")
            .and_then(|v| v.as_str())
            .or_else(|| op.get("type").and_then(|v| v.as_str()))
            .unwrap_or("");
        match kind {
            "~" | "update" => {
                let nid = op.get("id").and_then(|v| v.as_str()).ok_or("update missing id")?;
                let props = op.get("props").and_then(|v| v.as_object()).ok_or("update missing props")?;
                let node = t.get_mut(nid).ok_or("update: node not found")?;
                for (k, v) in props {
                    node.props.insert(k.clone(), v.clone());
                }
                t.dirty.insert(nid.to_string());
            }
            "+" | "insert" => {
                let anchor = op.get("anchor").and_then(|v| v.as_str()).unwrap_or("ui.root");
                let position = op.get("position").and_then(|v| v.as_str()).unwrap_or("child");
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
                let nid = op.get("id").and_then(|v| v.as_str()).ok_or("remove missing id")?;
                // detach from parent and drop subtree
                t.detach(nid);
                remove_subtree(&mut t, nid);
                t.dirty.insert(nid.to_string());
            }
            "!" | "replace" => {
                let nid = op.get("id").and_then(|v| v.as_str()).ok_or("replace missing id")?;
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
                let from = op.get("from").and_then(|v| v.as_str()).ok_or("move missing from")?;
                let to = op.get("to").and_then(|v| v.as_str()).ok_or("move missing to")?;
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
    t.dirty.clear();
    drop(t);

    // Reflect the updated subtree to the State Store (best-effort).
    let root = {
        let t = tree.lock().await;
        t.get(&t.root_id).cloned()
    };
    if let Some(node) = root {
        let _ = reflect_to_state(&node).await;
    }
    Ok(rev)
}

fn remove_subtree(t: &mut UiTree, id: &str) {
    if let Some(node) = t.nodes.remove(id) {
        for c in node.children {
            remove_subtree(t, &c);
        }
    }
}

/// Resolve an ASL token reference like "$colors.primary" against the theme.
pub fn resolve_token(token: &str, theme: &Theme) -> serde_json::Value {
    if let Some(rest) = token.strip_prefix('$') {
        let parts: Vec<&str> = rest.split('.').collect();
        let mut cur: serde_json::Value = serde_json::to_value(theme).unwrap_or(serde_json::Value::Null);
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
    mcp_call("state.set", serde_json::json!({ "path": "ui.root", "value": serialized })).await?;
    Some(())
}

// ---------------------------------------------------------------------------
// MCP client helper (talks to the bus).
// ---------------------------------------------------------------------------
async fn mcp_call(method: &str, params: serde_json::Value) -> Option<serde_json::Value> {
    let path = "/run/the-machine/mcp-bus.sock";
    let stream = tokio::net::UnixStream::connect(path).await.ok()?;
    let (mut reader, mut writer) = stream.into_split();
    let req = McpMessage::request(Uuid::new_v4(), method, Some(params));
    let bytes = serde_json::to_vec(&req).ok()?;
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
