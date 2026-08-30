//! Wayland session backend — G17 scaffold: bind `wl_display` before wlroots compositor wiring.
//! Pixel output remains DRM/framebuffer until wlroots seat/output/surface integration lands.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tracing::{info, warn};
use wayland_server::backend::ClientData;
use wayland_server::{Display, ListeningSocket};

use crate::drm;

/// How the compositor exposes Wayland to clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WaylandBackendMode {
    /// Set `WAYLAND_DISPLAY` only; pixel path stays DRM/framebuffer/memory.
    Marker,
    /// Bind a real `wl_display` socket via `wayland-server`.
    Display,
    /// Wayland session disabled for this process.
    Off,
}

/// Remaining wlroots integration steps after `wl_display` bind (G17 roadmap).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WlrootsInitPlan {
    pub steps: Vec<&'static str>,
    pub drm_available: bool,
    pub runtime_dir_set: bool,
    pub display_name: String,
}

impl WlrootsInitPlan {
    pub fn probe() -> Self {
        let display_name =
            std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into());
        Self {
            steps: vec![
                "bind wl_display socket (wayland-server)",
                "init wlroots backend + renderer",
                "create seat + output from DRM/KMS",
                "register xdg-shell and map compositor.surface MCP",
                "route input from system-daemon to focused surface",
            ],
            drm_available: drm::backend_available(),
            runtime_dir_set: runtime_dir().is_some(),
            display_name,
        }
    }
}

/// Resolve whether this run binds a real Wayland display or only publishes the env marker.
pub fn resolve_backend_mode() -> WaylandBackendMode {
    let backend = std::env::var("THE_MACHINE_COMPOSITOR_BACKEND").unwrap_or_else(|_| "auto".into());
    match backend.as_str() {
        "wayland" => WaylandBackendMode::Display,
        "memory" | "off" => WaylandBackendMode::Off,
        _ => WaylandBackendMode::Marker,
    }
}

fn runtime_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).or_else(|| {
        std::env::var("THE_MACHINE_SOCKET_DIR")
            .ok()
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
    })
}

fn ensure_runtime_dir() -> Option<PathBuf> {
    if let Some(dir) = runtime_dir() {
        let _ = std::fs::create_dir_all(&dir);
        if dir.is_dir() {
            if std::env::var_os("XDG_RUNTIME_DIR").is_none() {
                std::env::set_var("XDG_RUNTIME_DIR", &dir);
            }
            return Some(dir);
        }
    }
    None
}

pub fn try_start() -> Option<WaylandSession> {
    let mode = resolve_backend_mode();
    if mode == WaylandBackendMode::Off {
        info!("Wayland session disabled (THE_MACHINE_COMPOSITOR_BACKEND=off|memory)");
        return None;
    }

    let plan = WlrootsInitPlan::probe();
    let display_name = plan.display_name.clone();

    match mode {
        WaylandBackendMode::Marker => {
            if plan.drm_available {
                info!("WAYLAND_DISPLAY={display_name} (marker; DRM/KMS pixel backend active)");
            } else {
                info!("WAYLAND_DISPLAY={display_name} (marker; framebuffer/memory pixel backend)");
            }
            Some(WaylandSession {
                display_name,
                mode,
                socket_bound: false,
                init_plan: plan,
                guard: None,
            })
        }
        WaylandBackendMode::Display => match bind_display(&display_name) {
            Ok(guard) => {
                std::env::set_var("WAYLAND_DISPLAY", &display_name);
                info!("WAYLAND_DISPLAY={display_name} bound (wl_display socket active)");
                Some(WaylandSession {
                    display_name,
                    mode,
                    socket_bound: true,
                    init_plan: plan,
                    guard: Some(guard),
                })
            }
            Err(err) => {
                warn!(
                    "wl_display bind failed ({err}); falling back to WAYLAND_DISPLAY marker only"
                );
                std::env::set_var("WAYLAND_DISPLAY", &display_name);
                Some(WaylandSession {
                    display_name,
                    mode: WaylandBackendMode::Marker,
                    socket_bound: false,
                    init_plan: plan,
                    guard: None,
                })
            }
        },
        WaylandBackendMode::Off => None,
    }
}

pub struct WaylandSession {
    pub display_name: String,
    pub mode: WaylandBackendMode,
    pub socket_bound: bool,
    pub init_plan: WlrootsInitPlan,
    guard: Option<WaylandSessionGuard>,
}

impl WaylandSession {
    pub fn status_json(&self) -> serde_json::Value {
        serde_json::json!({
            "display": self.display_name,
            "mode": self.mode,
            "socket_bound": self.socket_bound,
            "wlroots_plan": self.init_plan,
        })
    }
}

struct WaylandSessionGuard {
    stop: Arc<AtomicBool>,
    _thread: thread::JoinHandle<()>,
}

struct BoundDisplay {
    display: Display<()>,
    socket: ListeningSocket,
}

fn bind_display(display_name: &str) -> Result<WaylandSessionGuard, String> {
    ensure_runtime_dir().ok_or_else(|| {
        "XDG_RUNTIME_DIR (or THE_MACHINE_SOCKET_DIR) must be set to bind wl_display".to_string()
    })?;

    let display = Display::<()>::new().map_err(|e| format!("Display::new: {e}"))?;
    let socket = ListeningSocket::bind(display_name).map_err(|e| format!("socket bind: {e}"))?;
    let bound = Arc::new(Mutex::new(BoundDisplay { display, socket }));

    let stop = Arc::new(AtomicBool::new(false));
    let stop_worker = stop.clone();
    let worker_bound = bound.clone();

    let thread = thread::Builder::new()
        .name("wayland-accept".into())
        .spawn(move || accept_loop(worker_bound, stop_worker))
        .map_err(|e| format!("spawn accept loop: {e}"))?;

    Ok(WaylandSessionGuard { stop, _thread: thread })
}

fn accept_loop(bound: Arc<Mutex<BoundDisplay>>, stop: Arc<AtomicBool>) {
    let client_data: Arc<dyn ClientData> = Arc::new(());
    while !stop.load(Ordering::Relaxed) {
        let accepted = {
            let guard = match bound.lock() {
                Ok(g) => g,
                Err(_) => break,
            };
            guard.socket.accept().ok().flatten()
        };
        if let Some(stream) = accepted {
            let guard = match bound.lock() {
                Ok(g) => g,
                Err(_) => break,
            };
            match guard.display.handle().insert_client(stream, client_data.clone()) {
                Ok(client) => info!("Wayland client connected: {:?}", client),
                Err(err) => warn!("Wayland client accept failed: {err}"),
            }
        } else {
            thread::sleep(Duration::from_millis(16));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_backend_mode_defaults_to_marker() {
        std::env::remove_var("THE_MACHINE_COMPOSITOR_BACKEND");
        assert_eq!(resolve_backend_mode(), WaylandBackendMode::Marker);
    }

    #[test]
    fn resolve_backend_mode_wayland_requests_display_bind() {
        std::env::set_var("THE_MACHINE_COMPOSITOR_BACKEND", "wayland");
        assert_eq!(resolve_backend_mode(), WaylandBackendMode::Display);
        std::env::remove_var("THE_MACHINE_COMPOSITOR_BACKEND");
    }

    #[test]
    fn wlroots_init_plan_lists_next_steps() {
        let plan = WlrootsInitPlan::probe();
        assert!(plan.steps.len() >= 4);
        assert!(plan.steps[0].contains("wl_display"));
        assert!(plan.steps.iter().any(|s| s.contains("wlroots")));
    }

    #[test]
    fn marker_session_when_backend_auto() {
        std::env::set_var("THE_MACHINE_COMPOSITOR_BACKEND", "auto");
        let session = try_start().expect("marker session");
        assert_eq!(session.mode, WaylandBackendMode::Marker);
        assert!(!session.socket_bound);
        std::env::remove_var("THE_MACHINE_COMPOSITOR_BACKEND");
    }

    #[test]
    fn display_bind_with_runtime_dir() {
        let dir = std::env::temp_dir().join(format!("tm-wl-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::env::set_var("XDG_RUNTIME_DIR", &dir);
        std::env::set_var("THE_MACHINE_COMPOSITOR_BACKEND", "wayland");
        std::env::set_var("WAYLAND_DISPLAY", "wayland-test");

        let session = try_start().expect("display session");
        assert_eq!(session.mode, WaylandBackendMode::Display);
        assert!(session.socket_bound);
        assert!(dir.join("wayland-test").exists());

        drop(session);
        std::env::remove_var("XDG_RUNTIME_DIR");
        std::env::remove_var("THE_MACHINE_COMPOSITOR_BACKEND");
        std::env::remove_var("WAYLAND_DISPLAY");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
