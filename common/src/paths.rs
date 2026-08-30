//! Runtime socket paths.
//!
//! Boot uses `/run/the-machine`. The development harness sets
//! `THE_MACHINE_SOCKET_DIR` (typically `/tmp/the-machine/run`) so daemons
//! do not need root to bind. Every listener and client must go through
//! these helpers so the two environments stay consistent.

/// Default socket directory on the ISO / initramfs boot path.
pub const DEFAULT_SOCKET_DIR: &str = "/run/the-machine";

/// Wayland runtime directory on boot / bare-metal installs.
pub const DEFAULT_RUNTIME_DIR: &str = "/run/the-machine";

/// DRM/KMS device node used by compositor and system-daemon display ops.
pub fn drm_device_path() -> String {
    std::env::var("THE_MACHINE_DRM_DEVICE").unwrap_or_else(|_| "/dev/dri/card0".into())
}

/// Directory that holds component Unix sockets.
pub fn socket_dir() -> String {
    std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| DEFAULT_SOCKET_DIR.to_string())
}

/// `{dir}/{name}.sock` with redundant trailing slashes stripped.
pub fn join_socket(dir: &str, name: &str) -> String {
    let dir = dir.trim_end_matches('/');
    let name = name.trim_start_matches('/').trim_end_matches(".sock");
    format!("{dir}/{name}.sock")
}

/// Socket path for a named component (`mcp-bus`, `system-daemon`, …).
pub fn component_socket(name: &str) -> String {
    join_socket(&socket_dir(), name)
}

/// Fast-path lease socket advertised by `bus.lease` / `lambda.lease`.
pub fn lease_socket(lease_id: &str) -> String {
    format!("{}/leases/{}.sock", socket_dir(), lease_id)
}

/// MCP bus socket. `THE_MACHINE_BUS_SOCKET` overrides the default
/// `{socket_dir}/mcp-bus.sock`.
pub fn bus_socket() -> String {
    std::env::var("THE_MACHINE_BUS_SOCKET").unwrap_or_else(|_| component_socket("mcp-bus"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_strips_suffix_and_slash() {
        assert_eq!(
            join_socket("/run/the-machine/", "mcp-bus.sock"),
            "/run/the-machine/mcp-bus.sock"
        );
        assert_eq!(
            join_socket("/tmp/the-machine/run", "system-daemon"),
            "/tmp/the-machine/run/system-daemon.sock"
        );
    }

    #[test]
    fn default_dir_is_boot_path() {
        // Only assert the constant — tests must not mutate process env.
        assert_eq!(DEFAULT_SOCKET_DIR, "/run/the-machine");
        assert_eq!(
            join_socket(DEFAULT_SOCKET_DIR, "fallback-shell"),
            "/run/the-machine/fallback-shell.sock"
        );
    }
}
