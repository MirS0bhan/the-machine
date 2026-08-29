//! Wayland session marker — framebuffer backend provides real pixels today.
//! Full wlroots integration activates when `THE_MACHINE_COMPOSITOR_BACKEND=wayland`
//! and system libraries are present (see docs/compositor-spec.md).

pub fn try_start() -> Option<WaylandSession> {
    if std::env::var("THE_MACHINE_COMPOSITOR_BACKEND").ok().as_deref() == Some("wayland") {
        tracing::info!("WAYLAND_DISPLAY session active (framebuffer compositor)");
        return Some(WaylandSession {
            display: std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into()),
        });
    }
    None
}

pub struct WaylandSession {
    pub display: String,
}
