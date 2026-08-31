//! Compositor — surface model + real pixel output (framebuffer / wlroots).

mod bitmap_font;
mod chrome;
mod drm;
mod env;
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
    std::env::set_var("WAYLAND_DISPLAY", crate::env::wayland_display_name());
    let pixels: SharedPixel = Arc::new(Mutex::new(PixelBackend::open()));
    let wayland: Arc<Option<wayland_backend::WaylandSession>> =
        Arc::new(wayland_backend::try_start(pixels.clone()));
    let comp: Arc<Mutex<Compositor>> = Arc::new(Mutex::new(Compositor::new()));

    // Background present loop (optional — disabled for static/test runs).
    if !crate::env::static_present_only() {
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
        tokio::time::sleep(std::time::Duration::from_millis(crate::env::frame_ms())).await;
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
    // Canvas = surface.canvas (dark theme) — not a TUI black void.
    let canvas = chrome::SURFACE_CANVAS;
    px.clear(canvas[0], canvas[1], canvas[2]);
    for s in &surfaces {
        paint_surface(&mut px, s);
    }
    px.present();
}

fn paint_surface(px: &mut PixelBackend, s: &Surface) {
    let w = s.geometry.width.max(1);
    let h = s.geometry.height.max(1);
    let radius = if s.radius > 0 {
        s.radius
    } else if matches!(s.kind.as_str(), "button" | "field" | "input") {
        chrome::RADIUS_MD
    } else {
        0
    };

    let bg = if s.confirmation {
        chrome::CONFIRMATION_BG
    } else if let Some(rgb) = s.bg {
        rgb
    } else {
        match s.kind.as_str() {
            "button" if s.variant == "primary" => chrome::ACCENT_DEFAULT,
            "button" => chrome::SURFACE_RAISED,
            "field" | "input" => chrome::SURFACE_SUNKEN,
            "text" => chrome::SURFACE_CANVAS,
            _ => {
                let (r, g, b) = hash_color(&s.id);
                [r, g, b]
            }
        }
    };

    let fg = s.fg.unwrap_or(match s.kind.as_str() {
        "button" if s.variant == "primary" || s.confirmation => chrome::TEXT_ON_ACCENT,
        "field" | "input" => chrome::TEXT_TERTIARY,
        _ => chrome::TEXT_PRIMARY,
    });

    // Text nodes are transparent chrome — label only, no filled plate.
    let draw_plate = s.kind != "text" || s.confirmation;
    if draw_plate {
        px.fill_rounded_rect(s.geometry.x, s.geometry.y, w, h, radius, bg);
        if let Some(border) = s.border {
            px.stroke_rounded_rect(s.geometry.x, s.geometry.y, w, h, radius, border);
        } else if matches!(s.kind.as_str(), "field" | "input") {
            let border = if s.focused {
                chrome::BORDER_FOCUS
            } else {
                chrome::BORDER_DEFAULT
            };
            px.stroke_rounded_rect(s.geometry.x, s.geometry.y, w, h, radius, border);
        }
    }

    if !s.label.is_empty() {
        let scale = s.font_scale.max(1);
        let pad_x = if draw_plate { 16i32 } else { 0 };
        let glyph_h = 7i32 * scale as i32;
        let pad_y = ((h as i32 - glyph_h) / 2).max(0);
        bitmap_font::draw_text_scaled(
            px,
            s.geometry.x + pad_x,
            s.geometry.y + pad_y,
            &s.label,
            fg[0],
            fg[1],
            fg[2],
            scale,
        );
    }
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
                bg: None,
                fg: None,
                border: None,
                radius: 0,
                font_scale: 2,
                variant: String::new(),
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
            if let Some(kind) = params.get("kind").and_then(|v| v.as_str()) {
                s.kind = kind.to_string();
            }
            if let Some(v) = params.get("variant").and_then(|v| v.as_str()) {
                s.variant = v.to_string();
            }
            if let Some(r) = params.get("radius").and_then(|v| v.as_u64()) {
                s.radius = r as u32;
            }
            if let Some(fs) = params.get("font_scale").and_then(|v| v.as_u64()) {
                s.font_scale = fs as u32;
            }
            s.bg = parse_rgb(params.get("bg")).or(s.bg);
            s.fg = parse_rgb(params.get("fg")).or(s.fg);
            s.border = parse_rgb(params.get("border")).or(s.border);
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
    for s in c.surfaces.values_mut() {
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

fn parse_rgb(v: Option<&serde_json::Value>) -> Option<[u8; 3]> {
    let v = v?;
    if let Some(arr) = v.as_array() {
        if arr.len() >= 3 {
            return Some([
                arr[0].as_u64().unwrap_or(0) as u8,
                arr[1].as_u64().unwrap_or(0) as u8,
                arr[2].as_u64().unwrap_or(0) as u8,
            ]);
        }
    }
    if let Some(s) = v.as_str() {
        let h = s.trim().trim_start_matches('#');
        if h.len() == 6 {
            let r = u8::from_str_radix(&h[0..2], 16).ok()?;
            let g = u8::from_str_radix(&h[2..4], 16).ok()?;
            let b = u8::from_str_radix(&h[4..6], 16).ok()?;
            return Some([r, g, b]);
        }
    }
    None
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

    fn test_harness() -> (
        Arc<Mutex<Compositor>>,
        SharedPixel,
        Arc<Option<wayland_backend::WaylandSession>>,
    ) {
        std::env::set_var("THE_MACHINE_COMPOSITOR_BACKEND", "memory");
        (
            Arc::new(Mutex::new(Compositor::new())),
            Arc::new(Mutex::new(PixelBackend::open())),
            Arc::new(None),
        )
    }

    #[tokio::test]
    async fn compositor_present_reports_memory_backend() {
        let (comp, pixels, wayland) = test_harness();
        let resp = handle_request(
            "compositor.present".into(),
            None,
            &comp,
            &pixels,
            &wayland,
        )
        .await;
        assert!(resp.error.is_none(), "unexpected {:?}", resp.error);
        let result = resp.result.expect("result");
        assert_eq!(result.get("presented").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(result.get("pixels").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(result.get("surfaces").and_then(|v| v.as_u64()), Some(0));
    }

    #[tokio::test]
    async fn compositor_surface_create_increments_present_count() {
        let (comp, pixels, wayland) = test_harness();
        let create = handle_request(
            "compositor.surface".into(),
            Some(serde_json::json!({
                "action": "create",
                "id": "widget.test",
                "geometry": { "x": 10, "y": 20, "width": 100, "height": 50 }
            })),
            &comp,
            &pixels,
            &wayland,
        )
        .await;
        assert!(create.error.is_none());
        let id = create
            .result
            .and_then(|r| r.get("id").and_then(|v| v.as_str()).map(str::to_string));
        assert_eq!(id.as_deref(), Some("widget.test"));

        let present = handle_request(
            "compositor.present".into(),
            None,
            &comp,
            &pixels,
            &wayland,
        )
        .await;
        let result = present.result.expect("present result");
        assert_eq!(result.get("surfaces").and_then(|v| v.as_u64()), Some(1));
    }

    #[tokio::test]
    async fn confirmation_set_active_promotes_surface() {
        let (comp, pixels, wayland) = test_harness();
        handle_request(
            "compositor.surface".into(),
            Some(serde_json::json!({
                "action": "create",
                "id": "confirm.me",
                "geometry": { "x": 0, "y": 0, "width": 80, "height": 40 }
            })),
            &comp,
            &pixels,
            &wayland,
        )
        .await;

        let activate = handle_request(
            "compositor.confirmation.set_active".into(),
            Some(serde_json::json!({ "active": true, "surface_id": "confirm.me" })),
            &comp,
            &pixels,
            &wayland,
        )
        .await;
        assert!(activate.error.is_none());
        let result = activate.result.expect("activate result");
        assert_eq!(result.get("active").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            result.get("surface_id").and_then(|v| v.as_str()),
            Some("confirm.me")
        );

        let c = comp.lock().await;
        assert!(c.confirmation_active);
        assert_eq!(c.confirmation_surface.as_deref(), Some("confirm.me"));
        let surface = c.surfaces.get("confirm.me").expect("surface");
        assert!(surface.confirmation);
        assert_eq!(surface.z_order, 10_000);
    }

    #[tokio::test]
    async fn styled_session_greeting_paints_design_system_shell() {
        let dump = "/tmp/compositor-session-greeting.ppm";
        std::env::set_var("THE_MACHINE_COMPOSITOR_BACKEND", "memory");
        std::env::set_var("THE_MACHINE_FB_WIDTH", "1280");
        std::env::set_var("THE_MACHINE_FB_HEIGHT", "720");
        std::env::set_var("THE_MACHINE_FB_DUMP", dump);
        let _ = std::fs::remove_file(dump);

        let (comp, pixels, wayland) = test_harness();
        let widgets = vec![
            serde_json::json!({
                "action": "create",
                "id": "surface.ui.greeting",
                "kind": "text",
                "label": "Welcome back",
                "geometry": { "x": 400, "y": 260, "width": 480, "height": 48 },
                "bg": [11, 12, 19],
                "fg": [247, 248, 252],
                "font_scale": 4,
                "radius": 0
            }),
            serde_json::json!({
                "action": "create",
                "id": "surface.ui.chat_input",
                "kind": "field",
                "label": "Ask or say what you need",
                "variant": "field",
                "geometry": { "x": 360, "y": 340, "width": 480, "height": 52 },
                "bg": [5, 5, 10],
                "fg": [130, 134, 156],
                "border": [59, 62, 82],
                "radius": 10,
                "font_scale": 3
            }),
            serde_json::json!({
                "action": "create",
                "id": "surface.ui.chat_send",
                "kind": "button",
                "label": "Send",
                "variant": "primary",
                "geometry": { "x": 560, "y": 410, "width": 96, "height": 48 },
                "bg": [156, 124, 242],
                "fg": [18, 19, 28],
                "radius": 10,
                "font_scale": 3
            }),
        ];
        for w in widgets {
            let resp = handle_request(
                "compositor.surface".into(),
                Some(w),
                &comp,
                &pixels,
                &wayland,
            )
            .await;
            assert!(resp.error.is_none(), "{:?}", resp.error);
        }
        let present = handle_request(
            "compositor.present".into(),
            None,
            &comp,
            &pixels,
            &wayland,
        )
        .await;
        assert!(present.error.is_none());
        assert!(std::path::Path::new(dump).exists());
        let bytes = std::fs::read(dump).expect("ppm");
        assert!(bytes.starts_with(b"P6"));
        let mut pos = 0usize;
        let mut newlines = 0u8;
        while pos < bytes.len() && newlines < 2 {
            if bytes[pos] == b'\n' {
                newlines += 1;
            }
            pos += 1;
        }
        let rgb = &bytes[pos..pos + 3];
        assert_eq!(
            rgb,
            &[0x0B, 0x0C, 0x13],
            "canvas must be design-system surface.canvas"
        );
        // Accent button fill should appear somewhere in the frame.
        let body = &bytes[pos..];
        let accent = [0x9C_u8, 0x7C, 0xF2];
        let has_accent = body.windows(3).any(|w| w == accent);
        assert!(has_accent, "primary button accent.default missing from framebuffer");
    }
}
