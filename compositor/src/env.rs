//! Compositor environment and display defaults.

pub const DEFAULT_FRAME_MS: u64 = 16;
pub const DEFAULT_WIDTH: u32 = 1920;
pub const DEFAULT_HEIGHT: u32 = 1080;
pub const DEFAULT_WAYLAND_DISPLAY: &str = "wayland-0";

pub fn compositor_backend() -> String {
    std::env::var("THE_MACHINE_COMPOSITOR_BACKEND").unwrap_or_else(|_| "auto".into())
}

pub fn frame_ms() -> u64 {
    std::env::var("THE_MACHINE_FRAME_MS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_FRAME_MS)
}

pub fn memory_framebuffer_size() -> (u32, u32) {
    let w = std::env::var("THE_MACHINE_FB_WIDTH")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1280);
    let h = std::env::var("THE_MACHINE_FB_HEIGHT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(720);
    (w, h)
}

/// When set, skip the background present loop (integration tests / static UI).
pub fn static_present_only() -> bool {
    matches!(
        std::env::var("THE_MACHINE_COMPOSITOR_STATIC").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE")
    )
}

pub fn wayland_display_name() -> String {
    std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| DEFAULT_WAYLAND_DISPLAY.into())
}

/// Whether this run should bind a real `wl_display` socket.
pub fn should_bind_wayland_display() -> bool {
    match compositor_backend().as_str() {
        "wayland" => true,
        "framebuffer" | "drm" | "memory" => false,
        _ => std::env::var("THE_MACHINE_WL_DISPLAY_BIND")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
    }
}
