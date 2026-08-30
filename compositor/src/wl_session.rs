//! Wayland display scaffold (G17) — binds `wl_display` via `wayland-server`.
//!
//! Registers `wl_compositor`, `wl_output`, and `wl_seat` globals so clients can
//! connect. Full surface commit → pixel paint remains future work.

use tracing::{info, warn};
use wayland_server::{BindError, Display, ListeningSocket};

use crate::wl_globals::{self, OutputInfo, WlGlobals};

/// Minimal compositor state for the Wayland display event loop.
#[derive(Debug, Default)]
pub struct CompositorState;

/// Active Wayland session: listening socket + display dispatch thread.
pub struct WlSession {
    pub display_name: String,
    _socket: ListeningSocket,
    _globals: WlGlobals,
}

/// Whether this run should bind a real `wl_display` socket.
pub fn should_bind_display() -> bool {
    match std::env::var("THE_MACHINE_COMPOSITOR_BACKEND")
        .unwrap_or_else(|_| "auto".into())
        .as_str()
    {
        "wayland" => true,
        "framebuffer" | "drm" | "memory" => false,
        _ => std::env::var("THE_MACHINE_WL_DISPLAY_BIND")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
    }
}

/// Resolve the preferred Wayland socket name before binding.
pub fn resolve_display_name() -> String {
    std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".into())
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

/// Bind `wl_display` and a listening socket when [`should_bind_display`] is true.
pub fn try_init() -> Option<WlSession> {
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

    let globals = WlGlobals::spawn(display, output);
    info!(
        "wl_display bound with compositor/output/seat globals; WAYLAND_DISPLAY={}",
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
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
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

        let session = try_init().expect("wl_display session");
        assert_eq!(session.display_name, "wayland-test");
        let status = session.status();
        assert!(status["bound"].as_bool().unwrap());
        assert!(status["globals"].is_array());

        drop(session);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
