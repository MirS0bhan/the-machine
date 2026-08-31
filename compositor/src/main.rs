//! Compositor — surface model + real pixel output (framebuffer / wlroots).

mod bitmap_font;
mod chrome;
mod clip;
mod damage;
mod drm;
mod env;
mod model;
mod pixel;
mod text;
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
    if std::env::var("THE_MACHINE_FONT_DIR").is_err() {
        if let Some(dir) = text::workspace_font_dir() {
            std::env::set_var("THE_MACHINE_FONT_DIR", dir);
        } else {
            for candidate in ["/etc/the-machine/fonts", "/the-machine/fonts"] {
                if std::path::Path::new(candidate).join("Inter-Regular.ttf").exists() {
                    std::env::set_var("THE_MACHINE_FONT_DIR", candidate);
                    break;
                }
            }
        }
    }
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
    let (surfaces, damage_frame) = {
        let mut c = comp.lock().await;
        let surfaces: Vec<Surface> = c
            .order
            .iter()
            .filter_map(|id| c.surfaces.get(id).cloned())
            .collect();
        let frame = c.damage.take();
        (surfaces, frame)
    };
    let mut px = pixels.lock().await;
    let canvas = chrome::SURFACE_CANVAS;
    if damage_frame.full || damage_frame.rects.is_empty() {
        px.clear(canvas[0], canvas[1], canvas[2]);
        for s in &surfaces {
            paint_surface(&mut px, s);
        }
    } else if let Some(bounds) = damage_frame.union_bounds() {
        // Partial present: clear union bounds then repaint intersecting surfaces.
        px.fill_rect(
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            canvas,
        );
        for s in &surfaces {
            let g = &s.geometry;
            let overlaps = g.x < bounds.x + bounds.width as i32
                && g.x + g.width as i32 > bounds.x
                && g.y < bounds.y + bounds.height as i32
                && g.y + g.height as i32 > bounds.y;
            if overlaps || s.kind == "dialog" {
                paint_surface(&mut px, s);
            }
        }
    } else {
        px.clear(canvas[0], canvas[1], canvas[2]);
        for s in &surfaces {
            paint_surface(&mut px, s);
        }
    }
    px.present();
}

fn paint_surface(px: &mut PixelBackend, s: &Surface) {
    let w = s.geometry.width.max(1);
    let h = s.geometry.height.max(1);
    let radius = if s.radius > 0 {
        s.radius
    } else if matches!(
        s.kind.as_str(),
        "button" | "field" | "input" | "toggle" | "dialog"
    ) {
        chrome::RADIUS_MD
    } else if s.kind == "slider" {
        chrome::RADIUS_SM
    } else {
        0
    };

    // Dialog: darken full framebuffer as scrim, then draw the card plate.
    if s.kind == "dialog" {
        px.fill_rect(0, 0, px.width(), px.height(), chrome::SCRIM);
    }

    let bg = if s.confirmation {
        chrome::CONFIRMATION_BG
    } else if s.pressed && s.kind == "button" && s.variant == "primary" {
        chrome::ACCENT_PRESSED
    } else if let Some(rgb) = s.bg {
        rgb
    } else {
        match s.kind.as_str() {
            "button" if s.variant == "primary" => chrome::ACCENT_DEFAULT,
            "button" => chrome::SURFACE_RAISED,
            "field" | "input" => chrome::SURFACE_SUNKEN,
            "text" => chrome::SURFACE_CANVAS,
            "toggle" if s.checked => chrome::ACCENT_DEFAULT,
            "toggle" => chrome::SURFACE_RAISED,
            "slider" => chrome::SURFACE_SUNKEN,
            "list" => chrome::SURFACE_CARD,
            "dialog" => chrome::SURFACE_OVERLAY,
            _ => {
                let (r, g, b) = hash_color(&s.id);
                [r, g, b]
            }
        }
    };

    let fg = s.fg.unwrap_or(match s.kind.as_str() {
        "button" if s.variant == "primary" || s.confirmation => chrome::TEXT_ON_ACCENT,
        "toggle" if s.checked => chrome::TEXT_ON_ACCENT,
        "field" | "input" => chrome::TEXT_TERTIARY,
        _ => chrome::TEXT_PRIMARY,
    });

    match s.kind.as_str() {
        "toggle" => {
            paint_toggle(px, s, bg, fg, radius);
            return;
        }
        "slider" => {
            paint_slider(px, s, bg, fg, radius);
            return;
        }
        "list" => {
            paint_list(px, s, bg, fg, radius);
            return;
        }
        "icon" => {
            paint_icon(px, s, bg, fg);
            return;
        }
        _ => {}
    }

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
        } else if s.pressed && s.kind == "button" {
            px.stroke_rounded_rect(
                s.geometry.x,
                s.geometry.y,
                w,
                h,
                radius,
                chrome::BORDER_FOCUS,
            );
        }
    }

    if !s.label.is_empty() {
        let font_px = text::resolve_px(s.font_px, s.font_scale);
        let weight = text::FontWeight::parse(&s.font_weight);
        let family = text::FontFamily::parse(&s.font_family);
        let pad_x = if draw_plate { 16i32 } else { 0 };
        let (_, text_h) = text::measure_text(&s.label, font_px, weight, family);
        let pad_y = ((h as i32 - text_h as i32) / 2).max(0);
        let text_x = s.geometry.x + pad_x;
        let text_y = s.geometry.y + pad_y;
        text::draw_text(px, text_x, text_y, &s.label, fg, font_px, weight, family);
        // Caret for focused fields.
        if s.focused && matches!(s.kind.as_str(), "field" | "input") && s.caret >= 0 {
            let (cx, cy, ch) = if s.placeholder_active || s.label.is_empty() {
                (text_x, text_y, text_h.max(font_px))
            } else {
                let caret = (s.caret as usize).min(s.label.len());
                let prefix = &s.label[..caret];
                let (prefix_w, _) = text::measure_text(prefix, font_px, weight, family);
                (text_x + prefix_w as i32, text_y, text_h.max(font_px))
            };
            px.fill_rect(cx, cy, 2, ch, chrome::BORDER_FOCUS);
        }
    } else if s.focused && matches!(s.kind.as_str(), "field" | "input") {
        // Empty field: caret at padding origin.
        let font_px = text::resolve_px(s.font_px, s.font_scale);
        let pad_x = 16i32;
        let pad_y = ((h as i32 - font_px as i32) / 2).max(0);
        px.fill_rect(
            s.geometry.x + pad_x,
            s.geometry.y + pad_y,
            2,
            font_px,
            chrome::BORDER_FOCUS,
        );
    }
}

fn paint_icon(px: &mut PixelBackend, s: &Surface, bg: [u8; 3], fg: [u8; 3]) {
    let w = s.geometry.width.max(1);
    let h = s.geometry.height.max(1);
    let size = w.min(h);
    let x = s.geometry.x + ((w as i32 - size as i32) / 2).max(0);
    let y = s.geometry.y + ((h as i32 - size as i32) / 2).max(0);
    // Soft plate behind glyph.
    px.fill_rounded_rect(x, y, size, size, chrome::RADIUS_SM, bg);
    let inset = (size / 5).max(2) as i32;
    let gw = size.saturating_sub((inset * 2) as u32).max(4);
    let gh = gw;
    let gx = x + inset;
    let gy = y + inset;
    match s.variant.as_str() {
        "check" => {
            // Simple check: two thick segments.
            px.fill_rect(gx, gy + gh as i32 / 2, gw / 3, 3, fg);
            px.fill_rect(
                gx + gw as i32 / 4,
                gy + gh as i32 / 2,
                3,
                gh / 2,
                fg,
            );
        }
        "close" | "x" => {
            // Approximate X with two bars (axis-aligned stand-in).
            px.fill_rect(gx, gy, gw, 3, fg);
            px.fill_rect(gx, gy + gh as i32 - 3, gw, 3, fg);
            px.fill_rect(gx + gw as i32 / 2 - 1, gy, 3, gh, fg);
        }
        _ => {
            // Default: bordered diamond-ish square.
            px.stroke_rounded_rect(gx, gy, gw, gh, 2, fg);
            let inner = (gw / 3).max(2);
            px.fill_rounded_rect(
                gx + (gw as i32 - inner as i32) / 2,
                gy + (gh as i32 - inner as i32) / 2,
                inner,
                inner,
                1,
                fg,
            );
        }
    }
}

fn paint_toggle(px: &mut PixelBackend, s: &Surface, bg: [u8; 3], fg: [u8; 3], radius: u32) {
    let w = s.geometry.width.max(1);
    let h = s.geometry.height.max(1);
    // Track
    px.fill_rounded_rect(s.geometry.x, s.geometry.y, w, h, radius, bg);
    px.stroke_rounded_rect(
        s.geometry.x,
        s.geometry.y,
        w,
        h,
        radius,
        s.border.unwrap_or(chrome::BORDER_DEFAULT),
    );
    // Knob
    let knob = (h as i32 - 8).max(12) as u32;
    let knob_x = if s.checked {
        s.geometry.x + w as i32 - knob as i32 - 4
    } else {
        s.geometry.x + 4
    };
    let knob_y = s.geometry.y + ((h as i32 - knob as i32) / 2).max(0);
    px.fill_rounded_rect(knob_x, knob_y, knob, knob, knob / 2, fg);
    if !s.label.is_empty() {
        let font_px = text::resolve_px(s.font_px, s.font_scale);
        let weight = text::FontWeight::parse(&s.font_weight);
        let family = text::FontFamily::parse(&s.font_family);
        text::draw_text(
            px,
            s.geometry.x + w as i32 + 10,
            s.geometry.y + ((h as i32 - font_px as i32) / 2).max(0),
            &s.label,
            chrome::TEXT_PRIMARY,
            font_px,
            weight,
            family,
        );
    }
}

fn paint_slider(px: &mut PixelBackend, s: &Surface, bg: [u8; 3], fg: [u8; 3], radius: u32) {
    let w = s.geometry.width.max(1);
    let h = s.geometry.height.max(1);
    let track_h = 6u32;
    let track_y = s.geometry.y + ((h as i32 - track_h as i32) / 2).max(0);
    px.fill_rounded_rect(s.geometry.x, track_y, w, track_h, radius, bg);
    let min = s.value_min;
    let max = if s.value_max > min {
        s.value_max
    } else {
        100.0
    };
    let t = ((s.value - min) / (max - min)).clamp(0.0, 1.0);
    let fill_w = ((w as f64) * t) as u32;
    if fill_w > 0 {
        px.fill_rounded_rect(s.geometry.x, track_y, fill_w, track_h, radius, fg);
    }
    let thumb = 16u32;
    let thumb_x = s.geometry.x + ((w.saturating_sub(thumb) as f64) * t) as i32;
    let thumb_y = s.geometry.y + ((h as i32 - thumb as i32) / 2).max(0);
    px.fill_rounded_rect(thumb_x, thumb_y, thumb, thumb, thumb / 2, fg);
}

fn paint_list(px: &mut PixelBackend, s: &Surface, bg: [u8; 3], fg: [u8; 3], radius: u32) {
    let w = s.geometry.width.max(1);
    let h = s.geometry.height.max(1);
    px.fill_rounded_rect(s.geometry.x, s.geometry.y, w, h, radius, bg);
    px.stroke_rounded_rect(
        s.geometry.x,
        s.geometry.y,
        w,
        h,
        radius,
        s.border.unwrap_or(chrome::BORDER_DEFAULT),
    );
    let clip = clip::ClipRect {
        x: s.geometry.x,
        y: s.geometry.y,
        width: w,
        height: h,
    };
    let rows: Vec<&str> = if !s.items.is_empty() {
        s.items.iter().map(|s| s.as_str()).collect()
    } else if !s.label.is_empty() {
        s.label.split('\n').collect()
    } else {
        Vec::new()
    };
    let row_h = 36i32;
    let font_px = text::resolve_px(s.font_px, s.font_scale);
    let weight = text::FontWeight::parse(&s.font_weight);
    let family = text::FontFamily::parse(&s.font_family);
    for (i, row) in rows.iter().enumerate() {
        let y = s.geometry.y + 4 + (i as i32) * row_h - s.scroll_y;
        if y + row_h < s.geometry.y || y > s.geometry.y + h as i32 {
            continue;
        }
        let row_bg = if i % 2 == 0 {
            chrome::SURFACE_CARD
        } else {
            chrome::SURFACE_RAISED
        };
        clip::fill_rect_clipped(
            px,
            Some(clip),
            s.geometry.x + 2,
            y,
            w.saturating_sub(4),
            (row_h as u32).saturating_sub(2),
            row_bg,
        );
        // Soft clip: only draw text if baseline is inside the list.
        if y >= s.geometry.y && y + font_px as i32 <= s.geometry.y + h as i32 {
            text::draw_text(
                px,
                s.geometry.x + 12,
                y + 8,
                row,
                fg,
                font_px,
                weight,
                family,
            );
        }
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
            let damage_mode = params
                .get("damage")
                .and_then(|v| v.as_str())
                .unwrap_or("full");
            {
                let mut c = comp.lock().await;
                if damage_mode == "full" {
                    c.damage.mark_full();
                }
            }
            paint_frame(comp, pixels).await;
            let c = comp.lock().await;
            success_response(
                &id,
                serde_json::json!({
                    "presented": true,
                    "surfaces": c.surfaces.len(),
                    "pixels": true,
                    "confirmation_active": c.confirmation_active,
                    "dialog_active": c.dialog_active,
                    "damage": damage_mode,
                    "backend": pixels.lock().await.backend_name(),
                    "text": text::backend_name(),
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
                    "text": text::backend_name(),
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
                pressed: false,
                label: String::new(),
                confirmation: false,
                checked: false,
                value: 0.0,
                value_min: 0.0,
                value_max: 100.0,
                scroll_y: 0,
                items: vec![],
                caret: -1,
                placeholder_active: false,
                bg: None,
                fg: None,
                border: None,
                radius: 0,
                font_scale: 2,
                font_px: 0,
                font_weight: "regular".into(),
                font_family: "default".into(),
                variant: String::new(),
            });
            s.id = sid.clone();
            if s.geometry.width == 0 {
                s.geometry.width = 200;
            }
            if s.geometry.height == 0 {
                s.geometry.height = 60;
            }
            apply_surface_fields(&mut s, &params);
            let mut c = comp.lock().await;
            c.damage.add(s.geometry.clone());
            if s.kind == "dialog" {
                c.dialog_active = true;
                c.dialog_surface = Some(sid.clone());
                s.z_order = s.z_order.max(5_000);
            }
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
            if let Some(old) = c.surfaces.remove(&sid) {
                c.damage.add(old.geometry.clone());
                if c.dialog_surface.as_ref() == Some(&sid) {
                    c.dialog_active = false;
                    c.dialog_surface = None;
                    c.damage.mark_full();
                }
            }
            c.recompute_order();
            drop(c);
            paint_frame(comp, pixels).await;
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
        "update" => {
            let sid = params
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let mut c = comp.lock().await;
            let patched = if let Some(s) = c.surfaces.get_mut(&sid) {
                let old_geo = s.geometry.clone();
                apply_surface_fields(s, &params);
                let new_geo = s.geometry.clone();
                let is_dialog = s.kind == "dialog";
                if is_dialog {
                    s.z_order = s.z_order.max(5_000);
                }
                Some((old_geo, new_geo, is_dialog))
            } else {
                None
            };
            if let Some((old_geo, new_geo, is_dialog)) = patched {
                c.damage.add_union(&old_geo, &new_geo);
                if is_dialog {
                    c.dialog_active = true;
                    c.dialog_surface = Some(sid.clone());
                }
                drop(c);
                paint_frame(comp, pixels).await;
                success_response(id, serde_json::json!({ "id": sid, "ok": true, "updated": true }))
            } else {
                error_response(id, "E_NOT_FOUND", "surface not found")
            }
        }
        _ => error_response(id, "E_INVALID", "unknown surface action"),
    }
}

fn apply_surface_fields(s: &mut Surface, params: &serde_json::Value) {
    if let Some(g) = params.get("geometry") {
        if let Ok(geo) = serde_json::from_value::<Geometry>(g.clone()) {
            s.geometry = geo;
        }
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
    if let Some(fp) = params.get("font_px").and_then(|v| v.as_u64()) {
        s.font_px = fp as u32;
    }
    if let Some(fw) = params.get("font_weight").and_then(|v| v.as_str()) {
        s.font_weight = fw.to_string();
    }
    if let Some(ff) = params.get("font_family").and_then(|v| v.as_str()) {
        s.font_family = ff.to_string();
    }
    if let Some(z) = params.get("z_order").and_then(|v| v.as_i64()) {
        s.z_order = z as i32;
    }
    if let Some(p) = params.get("pressed").and_then(|v| v.as_bool()) {
        s.pressed = p;
    }
    if let Some(p) = params.get("checked").and_then(|v| v.as_bool()) {
        s.checked = p;
    }
    if let Some(v) = params.get("value").and_then(|v| v.as_f64()) {
        s.value = v;
    }
    if let Some(v) = params.get("value_min").and_then(|v| v.as_f64()) {
        s.value_min = v;
    }
    if let Some(v) = params.get("value_max").and_then(|v| v.as_f64()) {
        s.value_max = v;
    }
    if let Some(v) = params.get("scroll_y").and_then(|v| v.as_i64()) {
        s.scroll_y = v as i32;
    }
    if let Some(arr) = params.get("items").and_then(|v| v.as_array()) {
        s.items = arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();
    }
    if let Some(c) = params.get("caret").and_then(|v| v.as_i64()) {
        s.caret = c;
    }
    if let Some(p) = params.get("placeholder_active").and_then(|v| v.as_bool()) {
        s.placeholder_active = p;
    }
    if let Some(bg) = parse_rgb(params.get("bg")) {
        s.bg = Some(bg);
    }
    if let Some(fg) = parse_rgb(params.get("fg")) {
        s.fg = Some(fg);
    }
    if let Some(border) = parse_rgb(params.get("border")) {
        s.border = Some(border);
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
    let event = params
        .get("event")
        .and_then(|v| v.as_str())
        .unwrap_or("click");
    if event == "key" {
        let c = comp.lock().await;
        let focused = c.focused.clone().or_else(|| {
            c.surfaces
                .iter()
                .find(|(_, s)| s.focused)
                .map(|(id, _)| id.clone())
        });
        let widget_id = focused
            .as_ref()
            .map(|sid| sid.strip_prefix("surface.").unwrap_or(sid).to_string());
        return success_response(
            id,
            serde_json::json!({
                "surface": focused,
                "widget_id": widget_id,
                "handled": focused.is_some(),
                "event": "key",
                "key": params.get("key"),
                "text": params.get("text"),
            }),
        );
    }
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
    let mut c = comp.lock().await;
    match c.pick(x, y) {
        Some(sid) => {
            let widget_id = sid.strip_prefix("surface.").unwrap_or(&sid).to_string();
            if event == "click" || event == "press" {
                for s in c.surfaces.values_mut() {
                    s.focused = false;
                    if event == "press" {
                        s.pressed = false;
                    }
                }
                if let Some(s) = c.surfaces.get_mut(&sid) {
                    s.focused = true;
                    if event == "press" {
                        s.pressed = true;
                    }
                }
                c.focused = Some(sid.clone());
                c.damage.mark_full();
            } else if event == "release" {
                for s in c.surfaces.values_mut() {
                    s.pressed = false;
                }
                c.damage.mark_full();
            }
            let (geometry, kind) = c
                .surfaces
                .get(&sid)
                .map(|s| (Some(s.geometry.clone()), Some(s.kind.clone())))
                .unwrap_or((None, None));
            success_response(
                id,
                serde_json::json!({
                    "surface": sid,
                    "widget_id": widget_id,
                    "handled": true,
                    "confirmation_only": c.confirmation_active,
                    "event": event,
                    "delta_y": params.get("delta_y"),
                    "geometry": geometry,
                    "kind": kind,
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
        if std::env::var("THE_MACHINE_FONT_DIR").is_err() {
            let font_dir = text::workspace_font_dir().expect("workspace assets/fonts");
            std::env::set_var(
                "THE_MACHINE_FONT_DIR",
                font_dir.display().to_string(),
            );
        }
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
                "font_px": 20,
                "font_weight": "bold",
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
                "font_scale": 3,
                "font_px": 14,
                "font_weight": "regular"
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
                "font_scale": 3,
                "font_px": 13,
                "font_weight": "medium"
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
        let result = present.result.expect("present");
        assert_eq!(
            result.get("text").and_then(|v| v.as_str()),
            Some("harfrust+freetype")
        );
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
