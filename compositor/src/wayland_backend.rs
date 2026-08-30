//! Wayland session marker — DRM/KMS dumb buffer is the primary GPU path today.
//! Set `THE_MACHINE_COMPOSITOR_BACKEND=drm|framebuffer|wayland|auto` (default: auto).
//!
//! G17: when backend is `wayland` (or `THE_MACHINE_WL_DISPLAY_BIND=1`), bind a real
//! `wl_display` socket via [`crate::wl_session`].

pub use crate::wl_session::{resolve_display_name, WlSession};

use crate::pixel::SharedPixel;

pub struct WaylandSession {
    pub display: String,
    pub wl: Option<WlSession>,
}

pub fn try_start(pixels: SharedPixel) -> Option<WaylandSession> {
    let backend = std::env::var("THE_MACHINE_COMPOSITOR_BACKEND").unwrap_or_else(|_| "auto".into());
    if !matches!(backend.as_str(), "auto" | "drm" | "wayland") {
        return None;
    }

    let wl = crate::wl_session::try_init(pixels);
    let display_name = wl
        .as_ref()
        .map(|s| s.display_name.clone())
        .unwrap_or_else(resolve_display_name);

    if wl.is_some() {
        tracing::info!("WAYLAND_DISPLAY={} (wl_display bound)", display_name);
    } else if super::drm::backend_available() {
        tracing::info!(
            "WAYLAND_DISPLAY={} with DRM/KMS pixel backend",
            display_name
        );
    } else {
        tracing::info!(
            "WAYLAND_DISPLAY={} (framebuffer/memory compositor)",
            display_name
        );
    }

    Some(WaylandSession {
        display: display_name,
        wl,
    })
}

impl WaylandSession {
    pub fn status(&self) -> serde_json::Value {
        match &self.wl {
            Some(wl) => wl.status(),
            None => serde_json::json!({
                "bound": false,
                "display": self.display,
                "engine": "marker",
                "wlroots": false,
            }),
        }
    }
}
