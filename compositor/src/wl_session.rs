//! Wayland display scaffold (G17) — binds `wl_display` via `wayland-server`.

use std::sync::{Arc, Mutex};

use tracing::{info, warn};
use wayland_server::{BindError, Display, ListeningSocket};

use crate::env;
use crate::pixel::SharedPixel;
use crate::wl_globals::{self, OutputInfo, WlGlobals};

/// Compositor state shared with the Wayland dispatch thread.
#[derive(Default)]
pub struct CompositorState {
    pub pixels: Option<SharedPixel>,
}

/// Active Wayland session: listening socket + display dispatch thread.
pub struct WlSession {
    pub display_name: String,
    _socket: ListeningSocket,
    _globals: WlGlobals,
}

pub fn should_bind_display() -> bool {
    env::should_bind_wayland_display()
}

pub fn resolve_display_name() -> String {
    env::wayland_display_name()
}

fn output_info() -> OutputInfo {
    if let Some((w, h)) = crate::drm::preferred_drm_size() {
        return OutputInfo {
            name: "drm-0".into(),
            width: w as i32,
            height: h as i32,
            refresh_mhz: 60_000,
        };
    }
    OutputInfo::default()
}

pub fn try_init(pixels: SharedPixel) -> Option<WlSession> {
    if !should_bind_display() {
        return None;
    }

    let preferred = resolve_display_name();
    let display = match Display::<CompositorState>::new() {
        Ok(d) => d,
        Err(e) => {
            warn!("wl_display create failed: {e}");
            return None;
        }
    };

    let output = output_info();
    if let Err(e) = wl_globals::register_globals(&display, output.clone()) {
        warn!("Wayland global registration failed: {e}");
        return None;
    }
    crate::wl_shm::register_shm_global(&display);
    crate::wl_xdg::register_xdg_shell_global(&display);

    let socket = match ListeningSocket::bind(&preferred) {
        Ok(s) => s,
        Err(BindError::AlreadyInUse) => match ListeningSocket::bind_auto("wayland", 0..32) {
            Ok(s) => s,
            Err(e) => {
                warn!("Wayland socket bind failed: {e}");
                return None;
            }
        },
        Err(e) => {
            warn!("Wayland socket bind failed: {e}");
            return None;
        }
    };

    let display_name = socket
        .socket_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or(preferred);
    std::env::set_var("WAYLAND_DISPLAY", &display_name);

    let state = Arc::new(Mutex::new(CompositorState {
        pixels: Some(pixels),
    }));
    let globals = WlGlobals::spawn(display, output, state);
    info!(
        "wl_display bound with compositor/output/seat/shm; WAYLAND_DISPLAY={}",
        display_name
    );

    Some(WlSession {
        display_name,
        _socket: socket,
        _globals: globals,
    })
}

impl WlSession {
    pub fn status(&self) -> serde_json::Value {
        let mut status = serde_json::json!({
            "bound": true,
            "display": self.display_name,
            "engine": "wayland-server",
            "surface_paint": true,
            "xdg_shell": "xdg_wm_base.v5",
            "xdg_toplevels": crate::wl_xdg::toplevel_count(),
            "wlroots": false,
        });
        if let Some(obj) = status.as_object_mut() {
            if let Some(globals) = self._globals.status().as_object() {
                for (k, v) in globals {
                    obj.insert(k.clone(), v.clone());
                }
            }
        }
        status
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex as StdMutex, OnceLock};

    fn env_lock() -> &'static StdMutex<()> {
        static LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| StdMutex::new(()))
    }

    struct EnvRestore(Vec<(String, Option<String>)>);

    impl EnvRestore {
        fn set(key: &str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self(vec![(key.to_string(), prev)])
        }

        fn unset(key: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::remove_var(key);
            Self(vec![(key.to_string(), prev)])
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, prev) in self.0.drain(..) {
                match prev {
                    Some(v) => std::env::set_var(&key, v),
                    None => std::env::remove_var(&key),
                }
            }
        }
    }

    #[test]
    fn should_bind_only_for_wayland_backend_or_opt_in() {
        let _guard = env_lock().lock().unwrap();
        let _a = EnvRestore::unset("THE_MACHINE_COMPOSITOR_BACKEND");
        let _b = EnvRestore::unset("THE_MACHINE_WL_DISPLAY_BIND");
        assert!(!should_bind_display());

        let _c = EnvRestore::set("THE_MACHINE_COMPOSITOR_BACKEND", "drm");
        assert!(!should_bind_display());

        let _d = EnvRestore::set("THE_MACHINE_COMPOSITOR_BACKEND", "wayland");
        assert!(should_bind_display());
    }

    #[test]
    fn resolve_display_name_defaults_to_wayland_0() {
        let _guard = env_lock().lock().unwrap();
        let _a = EnvRestore::unset("WAYLAND_DISPLAY");
        assert_eq!(resolve_display_name(), "wayland-0");
    }

    #[test]
    fn binds_display_in_temp_runtime_dir() {
        let _guard = env_lock().lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "the-machine-wl-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).expect("temp runtime dir");
        let _runtime = EnvRestore::set("XDG_RUNTIME_DIR", dir.to_str().unwrap());
        let _backend = EnvRestore::set("THE_MACHINE_COMPOSITOR_BACKEND", "wayland");
        let _display = EnvRestore::set("WAYLAND_DISPLAY", "wayland-test");

        let pixels =
            std::sync::Arc::new(tokio::sync::Mutex::new(crate::pixel::PixelBackend::open()));
        let session = try_init(pixels).expect("wl_display session");
        assert_eq!(session.display_name, "wayland-test");
        let status = session.status();
        assert!(status["bound"].as_bool().unwrap());
        assert!(status["globals"].is_array());

        drop(session);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
