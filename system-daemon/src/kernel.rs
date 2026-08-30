//! Kernel/hardware abstraction: power, display, network, audio.

use crate::{display, net};
use common::{AudioDevice, DisplayMode, NetworkInterface};

pub struct KernelHandler;

impl KernelHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn get_power_profile(&self) -> String {
        if let Ok(body) =
            std::fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
        {
            return body.trim().to_string();
        }
        "balanced".to_string()
    }

    pub fn get_display_modes(&self) -> Vec<DisplayMode> {
        display::get_display_modes()
    }

    pub fn list_interfaces(&self) -> Vec<NetworkInterface> {
        net::list_interfaces()
    }

    pub fn list_audio_devices(&self) -> Vec<AudioDevice> {
        vec![AudioDevice {
            name: "default".to_string(),
            r#type: "output".to_string(),
            default: true,
        }]
    }

    pub async fn set_power_profile(&self, _profile: &str) -> Result<(), String> {
        let path = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor";
        if !std::path::Path::new(path).exists() {
            return Ok(());
        }
        std::fs::write(path, format!("{_profile}\n"))
            .map_err(|e| format!("failed to set governor: {e}"))
    }

    pub async fn set_display_mode(
        &self,
        width: u32,
        height: u32,
        refresh: f32,
    ) -> Result<(), String> {
        display::set_display_mode(width, height, refresh)
    }

    pub async fn set_interface_state(&self, name: &str, state: &str) -> Result<(), String> {
        net::set_interface_state(name, state).await
    }

    pub fn get_wifi_status(&self) -> serde_json::Value {
        if let Ok(body) = std::fs::read_to_string("/proc/net/wireless") {
            for line in body.lines().skip(2) {
                let iface = line.split(':').next().unwrap_or("").trim();
                if iface.is_empty() {
                    continue;
                }
                return serde_json::json!({
                    "status": "associated",
                    "interface": iface,
                    "ssid": serde_json::Value::Null,
                    "source": "proc",
                });
            }
        }
        serde_json::json!({
            "status": "disconnected",
            "interface": serde_json::Value::Null,
            "ssid": serde_json::Value::Null,
        })
    }

    pub async fn connect_wifi(&self, _ssid: &str, _credential_ref: &str) -> Result<String, String> {
        Err("wifi connect requires wpa_supplicant adapter (not wired yet)".into())
    }

    pub async fn set_default_audio(&self, _name: &str) -> Result<(), String> {
        Err("audio.set_default requires PipeWire/ALSA session (not wired yet)".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wifi_status_reports_string_not_null() {
        let status = KernelHandler::new().get_wifi_status();
        assert!(status.get("status").and_then(|v| v.as_str()).is_some());
    }

    #[tokio::test]
    async fn connect_wifi_does_not_silently_succeed() {
        let err = KernelHandler::new()
            .connect_wifi("example", "cred-ref")
            .await
            .unwrap_err();
        assert!(err.contains("not wired"));
    }
}
