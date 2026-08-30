//! Network interface discovery and link state via rtnetlink (fallback: sysfs + `ip`).

use common::NetworkInterface;
use std::fs;
use std::path::Path;

use crate::cli::run_sync;
use crate::netlink;

pub(crate) fn reject_loopback_mutation(name: &str) -> Result<(), String> {
    if name == "lo" {
        Err("refusing to change loopback state".into())
    } else {
        Ok(())
    }
}

pub(crate) fn parse_link_state(state: &str) -> Result<bool, String> {
    match state {
        "up" => Ok(true),
        "down" => Ok(false),
        other => Err(format!("unsupported state: {other} (use up or down)")),
    }
}

pub(crate) fn classify_iface(name: &str, sysfs_path: &Path) -> &'static str {
    if name == "lo" {
        return "loopback";
    }
    if sysfs_path.join("wireless").exists() {
        return "wifi";
    }
    if fs::read_to_string(sysfs_path.join("type"))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        == Some(772)
    {
        return "loopback";
    }
    "ethernet"
}

pub async fn list_interfaces() -> Vec<NetworkInterface> {
    if let Ok(ifaces) = netlink::list_interfaces_netlink().await {
        return ifaces;
    }
    list_interfaces_sysfs()
}

pub fn list_interfaces_sysfs() -> Vec<NetworkInterface> {
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
        out.push(NetworkInterface {
            name: name.clone(),
            r#type: classify_iface(&name, &iface_path).to_string(),
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
    reject_loopback_mutation(name)?;
    if !Path::new(&format!("/sys/class/net/{name}")).exists() {
        return Err(format!("unknown interface: {name}"));
    }
    let up = parse_link_state(state)?;

    if let Ok(()) = netlink::set_interface_state_netlink(name, up).await {
        return Ok(());
    }
    set_interface_state_ip(name, state)
}

fn set_interface_state_ip(name: &str, state: &str) -> Result<(), String> {
    let action = match state {
        "up" => "up",
        "down" => "down",
        other => return Err(format!("unsupported state: {other} (use up or down)")),
    };
    run_sync("ip", &["link", "set", name, action]).map(|_| ())
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

    #[tokio::test]
    async fn list_interfaces_includes_loopback() {
        let ifaces = list_interfaces().await;
        assert!(ifaces.iter().any(|i| i.name == "lo"));
    }

    #[test]
    fn loopback_guard() {
        assert!(reject_loopback_mutation("lo").is_err());
    }
}
