//! Compositor - surface management, stacking, focus, input routing and damage.
//!
//! This is the model layer: it tracks logical surfaces (id, parent/child,
//! geometry, z-order, opacity, blur) and answers input-routing queries. It is
//! the single source of truth the UI Runtime composites against.

use common::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tracing::{info, warn, error};

#[derive(Clone, Serialize, Deserialize, Default)]
struct Geometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Clone, Serialize, Deserialize)]
struct Surface {
    id: String,
    #[serde(default)]
    parent: Option<String>,
    #[serde(default)]
    children: Vec<String>,
    #[serde(default)]
    geometry: Geometry,
    /// Higher = closer to the viewer.
    #[serde(default)]
    z_order: i32,
    #[serde(default = "default_one")]
    opacity: f32,
    #[serde(default)]
    blurred: bool,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    focused: bool,
}

fn default_one() -> f32 {
    1.0
}

struct Compositor {
    surfaces: HashMap<String, Surface>,
    /// Surfaces ordered by z_order (back to front).
    order: Vec<String>,
    focused: Option<String>,
}

impl Compositor {
    fn new() -> Self {
        Compositor {
            surfaces: HashMap::new(),
            order: Vec::new(),
            focused: None,
        }
    }

    fn recompute_order(&mut self) {
        let mut v: Vec<&String> = self.surfaces.keys().collect();
        v.sort_by_key(|id| self.surfaces[*id].z_order);
        self.order = v.into_iter().cloned().collect();
    }

    /// Topmost surface containing the point (x, y), or None.
    fn pick(&self, x: i32, y: i32) -> Option<String> {
        for id in self.order.iter().rev() {
            if let Some(s) = self.surfaces.get(id) {
                let g = &s.geometry;
                if x >= g.x && x < g.x + g.width as i32 && y >= g.y && y < g.y + g.height as i32 {
                    return Some(id.clone());
                }
            }
        }
        None
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    info!("Starting Compositor");
    let comp: Arc<Mutex<Compositor>> = Arc::new(Mutex::new(Compositor::new()));

    let socket_path = "/run/the-machine/compositor.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;
    info!("Compositor listening on {}", socket_path);

    loop {
        let (stream, _) = listener.accept().await?;
        let comp = comp.clone();
        tokio::spawn(async move {
            handle_connection(stream, comp).await;
        });
    }
}

async fn handle_connection(stream: tokio::net::UnixStream, comp: Arc<Mutex<Compositor>>) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                if let Ok(response) = process_message(&line, &comp).await {
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

async fn process_message(line: &str, comp: &Arc<Mutex<Compositor>>) -> anyhow::Result<String> {
    let msg: McpMessage = serde_json::from_str(line.trim())?;
    let id = msg.id;
    let response = match msg.kind {
        MessageKind::Request => {
            let method = msg.method.clone().unwrap_or_default();
            handle_request(method, msg.params, comp).await
        }
        _ => error_response(&id, "E_INVALID_REQUEST", "Only requests supported"),
    };
    Ok(serde_json::to_string(&response)? + "\n")
}

async fn handle_request(method: String, params: Option<serde_json::Value>, comp: &Arc<Mutex<Compositor>>) -> McpMessage {
    let id = Uuid::new_v4();
    let params = params.unwrap_or(serde_json::Value::Null);
    match method.as_str() {
        "compositor.surface" => {
            let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("create");
            match action {
                "create" => {
                    let sid = params.get("id").and_then(|v| v.as_str()).unwrap_or("surface").to_string();
                    let mut s = match serde_json::from_value::<Surface>(params.clone()) {
                        Ok(s) => s,
                        Err(_) => Surface {
                            id: sid.clone(),
                            parent: None,
                            children: Vec::new(),
                            geometry: Geometry::default(),
                            z_order: 0,
                            opacity: 1.0,
                            blurred: false,
                            kind: "window".to_string(),
                            focused: false,
                        },
                    };
                    s.id = sid.clone();
                    let mut c = comp.lock().await;
                    c.surfaces.insert(sid.clone(), s);
                    c.recompute_order();
                    success_response(&id, serde_json::json!({ "id": sid, "ok": true }))
                }
                "destroy" => {
                    let sid = params.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let mut c = comp.lock().await;
                    if let Some(s) = c.surfaces.remove(&sid) {
                        for child in s.children {
                            c.surfaces.remove(&child);
                        }
                        if let Some(p) = s.parent {
                            if let Some(p) = c.surfaces.get_mut(&p) {
                                p.children.retain(|x| x != &sid);
                            }
                        }
                        c.recompute_order();
                        success_response(&id, serde_json::json!({ "ok": true }))
                    } else {
                        error_response(&id, "E_NOT_FOUND", "surface not found")
                    }
                }
                "geometry" => {
                    let sid = params.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let g = params.get("geometry").cloned().unwrap_or(serde_json::Value::Null);
                    let mut c = comp.lock().await;
                    if let Some(s) = c.surfaces.get_mut(&sid) {
                        if let Ok(geo) = serde_json::from_value::<Geometry>(g) {
                            s.geometry = geo;
                            success_response(&id, serde_json::json!({ "ok": true }))
                        } else {
                            error_response(&id, "E_INVALID", "bad geometry")
                        }
                    } else {
                        error_response(&id, "E_NOT_FOUND", "surface not found")
                    }
                }
                _ => error_response(&id, "E_INVALID", "unknown surface action"),
            }
        }
        "compositor.blur" => {
            let sid = params.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let on = params.get("on").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut c = comp.lock().await;
            if let Some(s) = c.surfaces.get_mut(&sid) {
                s.blurred = on;
                success_response(&id, serde_json::json!({ "ok": true, "blurred": on }))
            } else {
                error_response(&id, "E_NOT_FOUND", "surface not found")
            }
        }
        "compositor.focus" => {
            let sid = params.get("id").and_then(|v| v.as_str()).map(|s| s.to_string());
            let mut c = comp.lock().await;
            for (_, s) in c.surfaces.iter_mut() {
                s.focused = false;
            }
            if let Some(sid) = sid {
                if let Some(s) = c.surfaces.get_mut(&sid) {
                    s.focused = true;
                    c.focused = Some(sid.clone());
                    success_response(&id, serde_json::json!({ "focused": sid }))
                } else {
                    error_response(&id, "E_NOT_FOUND", "surface not found")
                }
            } else {
                c.focused = None;
                success_response(&id, serde_json::json!({ "focused": serde_json::Value::Null }))
            }
        }
        "compositor.input" => {
            let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let c = comp.lock().await;
            match c.pick(x, y) {
                Some(sid) => success_response(&id, serde_json::json!({ "surface": sid, "handled": true })),
                None => success_response(&id, serde_json::json!({ "surface": serde_json::Value::Null, "handled": false })),
            }
        }
        "compositor.present" => {
            // Mark a surface's damage region as presented (model only).
            let sid = params.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let c = comp.lock().await;
            let ok = c.surfaces.contains_key(&sid);
            success_response(&id, serde_json::json!({ "presented": sid, "ok": ok }))
        }
        "compositor.list" => {
            let c = comp.lock().await;
            let surfaces: Vec<&Surface> = c.order.iter().map(|id| &c.surfaces[id]).collect();
            success_response(&id, serde_json::to_value(surfaces).unwrap_or(serde_json::Value::Null))
        }
        "compositor.status" => {
            let c = comp.lock().await;
            success_response(&id, serde_json::json!({
                "status": "running",
                "surfaces": c.surfaces.len(),
                "focused": c.focused,
            }))
        }
        _ => error_response(&id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
    }
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
