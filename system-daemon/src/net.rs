//! Network interface discovery and basic link state via sysfs + `ip`.

use common::NetworkInterface;
use std::fs;
use std::path::Path;

pub fn list_interfaces() -> Vec<NetworkInterface> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir("/sys/class/net") else {
        return fallback_interfaces();
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.is_empty() {
            continue;
        }
        let iface_path = entry.path();
        let state = fs::read_to_string(iface_path.join("operstate"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".into());
        let kind = if name == "lo" {
            "loopback"
        } else if iface_path.join("wireless").exists() {
            "wifi"
        } else if fs::read_to_string(iface_path.join("type"))
            .ok()
            .and_then(|s| s.trim().parse::<u32>().ok())
            == Some(772)
        {
            "loopback"
        } else {
            "ethernet"
        };
        out.push(NetworkInterface {
            name,
            r#type: kind.to_string(),
            state,
        });
    }
    if out.is_empty() {
        fallback_interfaces()
    } else {
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }
}

pub async fn set_interface_state(name: &str, state: &str) -> Result<(), String> {
    if name == "lo" {
        return Err("refusing to change loopback state".into());
    }
    if !Path::new(&format!("/sys/class/net/{name}")).exists() {
        return Err(format!("unknown interface: {name}"));
    }
    let action = match state {
        "up" => "up",
        "down" => "down",
        other => return Err(format!("unsupported state: {other} (use up or down)")),
    };
    let status = std::process::Command::new("ip")
        .args(["link", "set", name, action])
        .status()
        .map_err(|e| format!("ip link failed: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "ip link set {name} {action} failed (CAP_NET_ADMIN required)"
        ))
    }
}

fn fallback_interfaces() -> Vec<NetworkInterface> {
    vec![NetworkInterface {
        name: "lo".into(),
        r#type: "loopback".into(),
        state: "up".into(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_interfaces_includes_loopback() {
        let ifaces = list_interfaces();
        assert!(ifaces.iter().any(|i| i.name == "lo"));
    }
}
