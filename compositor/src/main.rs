//! Compositor — surface model + real pixel output (framebuffer / wlroots).

mod drm;
mod model;
mod pixel;
mod wayland_backend;
mod wl_globals;
mod wl_session;
mod wl_shm;

use common::*;
use model::{Compositor, Geometry, Surface};
use pixel::{hash_color, PixelBackend, SharedPixel};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    info!("Starting Compositor");
    std::env::set_var(
        "WAYLAND_DISPLAY",
        std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into()),
    );
    let pixels: SharedPixel = Arc::new(Mutex::new(PixelBackend::open()));
    let wayland: Arc<Option<wayland_backend::WaylandSession>> =
        Arc::new(wayland_backend::try_start(pixels.clone()));
    let comp: Arc<Mutex<Compositor>> = Arc::new(Mutex::new(Compositor::new()));

    // Background present loop.
    {
        let comp = comp.clone();
        let pixels = pixels.clone();
        tokio::spawn(async move {
            present_loop(comp, pixels).await;
        });
    }

    let socket_path = common::component_socket("compositor");
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    info!(
        "Compositor listening on {} (pixel backend active)",
        socket_path
    );

    loop {
        let (stream, _) = listener.accept().await?;
        let comp = comp.clone();
        let pixels = pixels.clone();
        let wayland = wayland.clone();
        tokio::spawn(async move {
            handle_connection(stream, comp, pixels, wayland).await;
        });
    }
}

async fn present_loop(comp: Arc<Mutex<Compositor>>, pixels: SharedPixel) {
    loop {
        paint_frame(&comp, &pixels).await;
        tokio::time::sleep(std::time::Duration::from_millis(16)).await;
    }
}

async fn paint_frame(comp: &Arc<Mutex<Compositor>>, pixels: &SharedPixel) {
    let surfaces: Vec<Surface> = {
        let c = comp.lock().await;
        c.order
            .iter()
            .filter_map(|id| c.surfaces.get(id).cloned())
            .collect()
    };
    let mut px = pixels.lock().await;
    px.clear(20, 24, 32);
    for s in &surfaces {
        let (r, g, b) = if s.confirmation {
            (200, 80, 60)
        } else {
            hash_color(&s.id)
        };
        px.fill_rect(
            s.geometry.x,
            s.geometry.y,
            s.geometry.width.max(40),
            s.geometry.height.max(24),
            r,
            g,
            b,
        );
    }
    px.present();
}

async fn handle_connection(
    stream: tokio::net::UnixStream,
    comp: Arc<Mutex<Compositor>>,
    pixels: SharedPixel,
    wayland: Arc<Option<wayland_backend::WaylandSession>>,
) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                if let Ok(response) = process_message(&line, &comp, &pixels, &wayland).await {
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

async fn process_message(
    line: &str,
    comp: &Arc<Mutex<Compositor>>,
    pixels: &SharedPixel,
    wayland: &Arc<Option<wayland_backend::WaylandSession>>,
) -> anyhow::Result<String> {
    let msg: McpMessage = serde_json::from_str(line.trim())?;
    let id = msg.id;
    let response = match msg.kind {
        MessageKind::Request => {
            let method = msg.method.clone().unwrap_or_default();
            handle_request(method, msg.params, comp, pixels, wayland).await
        }
        _ => error_response(&id, "E_INVALID_REQUEST", "Only requests supported"),
    };
    Ok(serde_json::to_string(&response)? + "\n")
}

async fn handle_request(
    method: String,
    params: Option<serde_json::Value>,
    comp: &Arc<Mutex<Compositor>>,
    pixels: &SharedPixel,
    wayland: &Arc<Option<wayland_backend::WaylandSession>>,
) -> McpMessage {
    let id = Uuid::new_v4();
    let params = params.unwrap_or(serde_json::Value::Null);
    match method.as_str() {
        "compositor.surface" => handle_surface(params, comp, pixels, &id).await,
        "compositor.confirmation.set_active" => {
            let active = params
                .get("active")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let surface_id = params
                .get("surface_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let mut c = comp.lock().await;
            c.confirmation_active = active;
            c.confirmation_surface = if active { surface_id.clone() } else { None };
            if let Some(sid) = surface_id {
                if let Some(s) = c.surfaces.get_mut(&sid) {
                    s.confirmation = active;
                    s.z_order = if active { 10_000 } else { s.z_order };
                }
            }
            c.recompute_order();
            success_response(
                &id,
                serde_json::json!({ "active": active, "surface_id": c.confirmation_surface }),
            )
        }
        "compositor.blur" => {
            let sid = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let on = params.get("on").and_then(|v| v.as_bool()).unwrap_or(true);
            let mut c = comp.lock().await;
            if let Some(s) = c.surfaces.get_mut(&sid) {
                s.blurred = on;
                success_response(&id, serde_json::json!({ "ok": true, "blurred": on }))
            } else {
                error_response(&id, "E_NOT_FOUND", "surface not found")
            }
        }
        "compositor.focus" => handle_focus(params, comp, &id).await,
        "compositor.input" => handle_input(params, comp, &id).await,
        "compositor.present" => {
            paint_frame(comp, pixels).await;
            let c = comp.lock().await;
            success_response(
                &id,
                serde_json::json!({
                    "presented": true,
                    "surfaces": c.surfaces.len(),
                    "pixels": true,
                    "confirmation_active": c.confirmation_active,
                    "backend": pixels.lock().await.backend_name(),
                }),
            )
        }
        "compositor.list" => {
            let c = comp.lock().await;
            let surfaces: Vec<&Surface> = c.order.iter().map(|id| &c.surfaces[id]).collect();
            success_response(
                &id,
                serde_json::to_value(surfaces).unwrap_or(serde_json::Value::Null),
            )
        }
        "compositor.status" => {
            let c = comp.lock().await;
            let backend = pixels.lock().await.backend_name();
            let wayland_session = wayland
                .as_ref()
                .as_ref()
                .map(wayland_backend::WaylandSession::status)
                .unwrap_or(serde_json::Value::Null);
            success_response(
                &id,
                serde_json::json!({
                    "status": "running",
                    "surfaces": c.surfaces.len(),
                    "focused": c.focused,
                    "pixels": true,
                    "confirmation_active": c.confirmation_active,
                    "backend": backend,
                    "wayland_session": wayland_session,
                }),
            )
        }
        _ => error_response(&id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
    }
}

async fn handle_surface(
    params: serde_json::Value,
    comp: &Arc<Mutex<Compositor>>,
    pixels: &SharedPixel,
    id: &Uuid,
) -> McpMessage {
    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("create");
    match action {
        "create" => {
            let sid = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("surface")
                .to_string();
            let mut s: Surface = serde_json::from_value(params.clone()).unwrap_or(Surface {
                id: sid.clone(),
                parent: None,
                children: vec![],
                geometry: Geometry::default(),
                z_order: 0,
                opacity: 1.0,
                blurred: false,
                kind: "window".into(),
                focused: false,
                label: String::new(),
                confirmation: false,
            });
            s.id = sid.clone();
            if s.geometry.width == 0 {
                s.geometry.width = 200;
            }
            if s.geometry.height == 0 {
                s.geometry.height = 60;
            }
            if let Some(label) = params.get("label").and_then(|v| v.as_str()) {
                s.label = label.to_string();
            }
            let mut c = comp.lock().await;
            c.surfaces.insert(sid.clone(), s);
            c.recompute_order();
            drop(c);
            paint_frame(comp, pixels).await;
            success_response(
                id,
                serde_json::json!({ "id": sid, "ok": true, "pixels": true }),
            )
        }
        "destroy" => {
            let sid = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut c = comp.lock().await;
            c.surfaces.remove(&sid);
            c.recompute_order();
            success_response(id, serde_json::json!({ "ok": true }))
        }
        "geometry" => {
            let sid = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let g = params
                .get("geometry")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let mut c = comp.lock().await;
            if let Some(s) = c.surfaces.get_mut(&sid) {
                if let Ok(geo) = serde_json::from_value::<Geometry>(g) {
                    s.geometry = geo;
                    success_response(id, serde_json::json!({ "ok": true }))
                } else {
                    error_response(id, "E_INVALID", "bad geometry")
                }
            } else {
                error_response(id, "E_NOT_FOUND", "surface not found")
            }
        }
        _ => error_response(id, "E_INVALID", "unknown surface action"),
    }
}

async fn handle_focus(
    params: serde_json::Value,
    comp: &Arc<Mutex<Compositor>>,
    id: &Uuid,
) -> McpMessage {
    let sid = params
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut c = comp.lock().await;
    for (_, s) in c.surfaces.iter_mut() {
        s.focused = false;
    }
    if let Some(sid) = sid {
        if let Some(s) = c.surfaces.get_mut(&sid) {
            s.focused = true;
            c.focused = Some(sid.clone());
            success_response(id, serde_json::json!({ "focused": sid }))
        } else {
            error_response(id, "E_NOT_FOUND", "surface not found")
        }
    } else {
        c.focused = None;
        success_response(
            id,
            serde_json::json!({ "focused": serde_json::Value::Null }),
        )
    }
}

async fn handle_input(
    params: serde_json::Value,
    comp: &Arc<Mutex<Compositor>>,
    id: &Uuid,
) -> McpMessage {
    let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let require_provenance = params
        .get("require_provenance")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if require_provenance && params.get("provenance").is_none() {
        return success_response(
            id,
            serde_json::json!({ "handled": false, "reason": "provenance_required" }),
        );
    }
    let c = comp.lock().await;
    match c.pick(x, y) {
        Some(sid) => {
            let widget_id = sid.strip_prefix("surface.").unwrap_or(&sid).to_string();
            success_response(
                id,
                serde_json::json!({
                    "surface": sid,
                    "widget_id": widget_id,
                    "handled": true,
                    "confirmation_only": c.confirmation_active,
                }),
            )
        }
        None => success_response(
            id,
            serde_json::json!({
                "surface": serde_json::Value::Null,
                "widget_id": serde_json::Value::Null,
                "handled": false,
                "confirmation_active": c.confirmation_active,
            }),
        ),
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
