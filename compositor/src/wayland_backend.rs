//! Wayland / wlroots session scaffold — DRM/KMS dumb buffer is the active pixel path today.
//!
//! Set `THE_MACHINE_COMPOSITOR_BACKEND=auto|framebuffer|drm|wayland|wlroots` (default: auto).
//! The wlroots backend records an init plan and runtime preconditions; it does not yet own
//! `wl_display` (G17 sub-step — full session is a follow-up PR).

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositorBackend {
    Framebuffer,
    Drm,
    Wlroots,
}

impl CompositorBackend {
    pub fn as_str(self) -> &'static str {
        match self {
            CompositorBackend::Framebuffer => "framebuffer",
            CompositorBackend::Drm => "drm",
            CompositorBackend::Wlroots => "wlroots",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendPreference {
    Auto,
    Framebuffer,
    Drm,
    WaylandCompat,
    Wlroots,
}

pub fn parse_backend_env(raw: Option<&str>) -> BackendPreference {
    match raw.unwrap_or("auto").trim().to_ascii_lowercase().as_str() {
        "framebuffer" | "fb" | "memory" => BackendPreference::Framebuffer,
        "drm" | "kms" => BackendPreference::Drm,
        "wayland" => BackendPreference::WaylandCompat,
        "wlroots" => BackendPreference::Wlroots,
        _ => BackendPreference::Auto,
    }
}

pub fn select_backend(preference: BackendPreference, drm_available: bool) -> CompositorBackend {
    match preference {
        BackendPreference::Framebuffer => CompositorBackend::Framebuffer,
        BackendPreference::Drm => {
            if drm_available {
                CompositorBackend::Drm
            } else {
                CompositorBackend::Framebuffer
            }
        }
        BackendPreference::WaylandCompat | BackendPreference::Wlroots => CompositorBackend::Wlroots,
        BackendPreference::Auto => {
            if drm_available {
                CompositorBackend::Drm
            } else {
                CompositorBackend::Framebuffer
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WlrootsScaffold {
    pub display_name: String,
    pub socket_path: PathBuf,
    pub runtime_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WlrootsProbe {
    pub ready: bool,
    pub missing: Vec<&'static str>,
}

impl WlrootsScaffold {
    pub fn from_env() -> Self {
        let display_name =
            std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("THE_MACHINE_SOCKET_DIR")
                    .map(PathBuf::from)
                    .map(|p| p.join("wayland"))
            })
            .unwrap_or_else(|_| PathBuf::from("/run/the-machine/wayland"));
        let socket_path = runtime_dir.join(&display_name);
        WlrootsScaffold {
            display_name,
            socket_path,
            runtime_dir,
        }
    }

    pub fn probe(&self) -> WlrootsProbe {
        let mut missing = Vec::new();
        if !self.runtime_dir.is_dir() {
            missing.push("runtime_dir");
        }
        if self.display_name.is_empty() {
            missing.push("display_name");
        }
        if !super::drm::backend_available() {
            missing.push("drm_device");
        }
        WlrootsProbe {
            ready: missing.is_empty(),
            missing,
        }
    }

    pub fn init_plan(&self) -> Vec<&'static str> {
        vec![
            "create runtime dir with 0700 permissions",
            "wlroots::Display::new() and bind display socket",
            "wlr_backend_autocreate + wlr_renderer_autocreate",
            "wlr_compositor + wlr_xdg_shell + seat from system-daemon input",
            "map compositor.surface MCP tree to xdg toplevels",
        ]
    }
}

#[derive(Clone)]
pub struct WaylandSession {
    pub display: String,
    pub backend: CompositorBackend,
    pub wlroots: Option<WlrootsScaffold>,
    pub wlroots_ready: bool,
}

pub fn try_start() -> Option<WaylandSession> {
    let preference = parse_backend_env(std::env::var("THE_MACHINE_COMPOSITOR_BACKEND").ok().as_deref());
    let drm_available = super::drm::backend_available();
    let backend = select_backend(preference, drm_available);
    let display_name = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());

    let (wlroots, wlroots_ready) = if backend == CompositorBackend::Wlroots {
        let scaffold = WlrootsScaffold::from_env();
        let probe = scaffold.probe();
        if probe.ready {
            tracing::info!(
                display = %scaffold.display_name,
                socket = %scaffold.socket_path.display(),
                "wlroots scaffold preconditions satisfied (session init pending)"
            );
        } else {
            tracing::warn!(
                display = %scaffold.display_name,
                missing = ?probe.missing,
                "wlroots scaffold not ready; pixel backend remains authoritative"
            );
        }
        (Some(scaffold), probe.ready)
    } else {
        (None, false)
    };

    match backend {
        CompositorBackend::Drm => {
            tracing::info!("WAYLAND_DISPLAY={} with DRM/KMS backend", display_name);
        }
        CompositorBackend::Framebuffer => {
            tracing::info!("WAYLAND_DISPLAY={} (framebuffer/memory compositor)", display_name);
        }
        CompositorBackend::Wlroots => {
            tracing::info!(
                "WAYLAND_DISPLAY={} (wlroots scaffold; init plan has {} steps)",
                display_name,
                wlroots
                    .as_ref()
                    .map(|s| s.init_plan().len())
                    .unwrap_or(0)
            );
        }
    }

    Some(WaylandSession {
        display: display_name,
        backend,
        wlroots,
        wlroots_ready,
    })
}

pub fn socket_path_for_display(display: &str, runtime_dir: &Path) -> PathBuf {
    runtime_dir.join(display)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_backend_env_recognizes_wlroots() {
        assert_eq!(parse_backend_env(Some("wlroots")), BackendPreference::Wlroots);
        assert_eq!(parse_backend_env(Some("WAYLAND")), BackendPreference::WaylandCompat);
        assert_eq!(parse_backend_env(Some("auto")), BackendPreference::Auto);
    }

    #[test]
    fn select_backend_auto_prefers_drm_when_available() {
        assert_eq!(
            select_backend(BackendPreference::Auto, true),
            CompositorBackend::Drm
        );
        assert_eq!(
            select_backend(BackendPreference::Auto, false),
            CompositorBackend::Framebuffer
        );
    }

    #[test]
    fn select_backend_wlroots_is_distinct_from_drm_auto() {
        assert_eq!(
            select_backend(BackendPreference::Wlroots, true),
            CompositorBackend::Wlroots
        );
    }

    #[test]
    fn wlroots_scaffold_builds_socket_path() {
        let runtime = PathBuf::from("/tmp/the-machine/wayland");
        let socket = socket_path_for_display("wayland-0", &runtime);
        assert_eq!(socket, runtime.join("wayland-0"));
    }

    #[test]
    fn wlroots_probe_reports_missing_runtime_dir() {
        let scaffold = WlrootsScaffold {
            display_name: "wayland-0".into(),
            socket_path: PathBuf::from("/no/such/wayland-0"),
            runtime_dir: PathBuf::from("/no/such/runtime"),
        };
        let probe = scaffold.probe();
        assert!(!probe.ready);
        assert!(probe.missing.contains(&"runtime_dir"));
    }

    #[test]
    fn wlroots_init_plan_lists_session_steps() {
        let scaffold = WlrootsScaffold {
            display_name: "wayland-0".into(),
            socket_path: PathBuf::from("/run/the-machine/wayland/wayland-0"),
            runtime_dir: PathBuf::from("/run/the-machine/wayland"),
        };
        let plan = scaffold.init_plan();
        assert!(plan.len() >= 4);
        assert!(plan[0].contains("runtime dir"));
    }
}
