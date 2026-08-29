//! Wayland session marker — DRM/KMS dumb buffer is the primary GPU path today.
//! Set `THE_MACHINE_COMPOSITOR_BACKEND=drm|framebuffer|auto` (default: auto).

pub fn try_start() -> Option<WaylandSession> {
    let backend = std::env::var("THE_MACHINE_COMPOSITOR_BACKEND").unwrap_or_else(|_| "auto".into());
    if matches!(backend.as_str(), "auto" | "drm" | "wayland") {
        let display_name = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
        if super::drm::backend_available() {
            tracing::info!("WAYLAND_DISPLAY={} with DRM/KMS backend", display_name);
        } else {
            tracing::info!("WAYLAND_DISPLAY={} (framebuffer/memory compositor)", display_name);
        }
        return Some(WaylandSession { display: display_name });
    }
    None
}

pub struct WaylandSession {
    pub display: String,
}
