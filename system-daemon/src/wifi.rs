//! Wi-Fi via `wpa_supplicant` / `wpa_cli` on bare-metal hosts.

use std::path::Path;
use tokio::process::Command;

const SECRET_DIRS: &[&str] = &["/run/the-machine/secrets", "/etc/the-machine/secrets"];

fn resolve_credential(credential_ref: &str) -> Result<String, String> {
    if credential_ref.is_empty() {
        return Err("credential_ref is required for wifi connect".into());
    }
    if credential_ref.contains('/') || credential_ref.contains("..") {
        return Err("invalid credential_ref".into());
    }
    for dir in SECRET_DIRS {
        let path = Path::new(dir).join(credential_ref);
        if path.is_file() {
            return std::fs::read_to_string(&path)
                .map(|s| s.trim().to_string())
                .map_err(|e| format!("failed to read credential: {e}"));
        }
    }
    Err(format!(
        "credential_ref not found in {}",
        SECRET_DIRS.join(" or ")
    ))
}

fn wifi_interface() -> Option<String> {
    let wireless = std::fs::read_to_string("/proc/net/wireless").ok()?;
    for line in wireless.lines().skip(2) {
        let iface = line.split(':').next()?.trim();
        if !iface.is_empty() {
            return Some(iface.to_string());
        }
    }
    None
}

async fn wpa_cli(iface: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new("wpa_cli")
        .arg("-i")
        .arg(iface)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("wpa_cli not available: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("wpa_cli failed: {stdout} {stderr}"));
    }
    Ok(stdout)
}

/// Connect to a Wi-Fi network using wpa_supplicant.
pub async fn connect_wifi(ssid: &str, credential_ref: &str) -> Result<String, String> {
    let iface = wifi_interface()
        .ok_or_else(|| "no wireless interface found (check /proc/net/wireless)".to_string())?;
    let psk = resolve_credential(credential_ref)?;

    let net_id = wpa_cli(&iface, &["add_network"]).await?;
    let net_id = net_id.trim();
    wpa_cli(
        &iface,
        &["set_network", net_id, "ssid", &format!("\"{ssid}\"")],
    )
    .await?;
    wpa_cli(
        &iface,
        &["set_network", net_id, "psk", &format!("\"{psk}\"")],
    )
    .await?;
    wpa_cli(&iface, &["enable_network", net_id]).await?;
    wpa_cli(&iface, &["save_config"]).await?;
    wpa_cli(&iface, &["reconnect"]).await?;

    Ok("associating".to_string())
}

pub fn wifi_status() -> serde_json::Value {
    if let Some(iface) = wifi_interface() {
        if let Ok(body) = std::fs::read_to_string(format!("/sys/class/net/{iface}/operstate")) {
            let state = body.trim();
            let status = if state == "up" {
                "associated"
            } else {
                "disconnected"
            };
            return serde_json::json!({
                "status": status,
                "interface": iface,
                "ssid": serde_json::Value::Null,
                "source": "sysfs",
            });
        }
        return serde_json::json!({
            "status": "disconnected",
            "interface": iface,
            "ssid": serde_json::Value::Null,
            "source": "proc",
        });
    }
    serde_json::json!({
        "status": "disconnected",
        "interface": serde_json::Value::Null,
        "ssid": serde_json::Value::Null,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal_in_credential_ref() {
        assert!(resolve_credential("../etc/passwd").is_err());
    }

    #[test]
    fn wifi_status_has_status_field() {
        let status = wifi_status();
        assert!(status.get("status").and_then(|v| v.as_str()).is_some());
    }
}
