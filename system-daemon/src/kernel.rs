//! Kernel/hardware abstraction: power, display, network, audio.

use crate::power;
use common::{AudioDevice, DisplayMode, NetworkInterface};

pub struct KernelHandler;

impl KernelHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn get_power_profile(&self) -> String {
        power::read_power_profile().unwrap_or_else(|| "balanced".to_string())
    }

    pub fn get_display_modes(&self) -> Vec<DisplayMode> {
        vec![DisplayMode {
            width: 1920,
            height: 1080,
            refresh: 60.0,
            current: true,
        }]
    }

    pub fn list_interfaces(&self) -> Vec<NetworkInterface> {
        vec![NetworkInterface {
            name: "lo".to_string(),
            r#type: "loopback".to_string(),
            state: "up".to_string(),
        }]
    }

    pub fn list_audio_devices(&self) -> Vec<AudioDevice> {
        vec![AudioDevice {
            name: "default".to_string(),
            r#type: "output".to_string(),
            default: true,
        }]
    }

    pub async fn set_power_profile(&self, profile: &str) -> Result<(), String> {
        power::write_power_profile(profile)
    }

    pub async fn set_display_mode(
        &self,
        _width: u32,
        _height: u32,
        _refresh: f32,
    ) -> Result<(), String> {
        Ok(())
    }

    pub async fn set_interface_state(&self, _name: &str, _state: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn get_wifi_status(&self) -> serde_json::Value {
        // /proc/net/wireless exists when a wireless stack is loaded.
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
        // Mutation path is gated by grant tokens; the host wpa_supplicant
        // adapter is not wired yet, so report an honest status instead of
        // serializing `()` as JSON null.
        Err("wifi connect is not wired on this host".into())
    }

    pub async fn set_default_audio(&self, _name: &str) -> Result<(), String> {
        Ok(())
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

    #[test]
    fn power_profile_read_returns_known_value() {
        let profile = KernelHandler::new().get_power_profile();
        assert!(matches!(
            profile.as_str(),
            "balanced" | "performance" | "powersave"
        ));
    }

    #[tokio::test]
    async fn set_power_profile_rejects_invalid_name() {
        let err = KernelHandler::new()
            .set_power_profile("turbo")
            .await
            .unwrap_err();
        assert!(err.contains("unsupported profile"));
    }
}
