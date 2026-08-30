//! Kernel/hardware abstraction: power, display, network, audio.

use crate::{audio, display, net, power, wifi};
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
        display::get_display_modes()
    }

    pub async fn list_interfaces(&self) -> Vec<NetworkInterface> {
        net::list_interfaces().await
    }

    pub fn list_audio_devices(&self) -> Vec<AudioDevice> {
        audio::list_audio_devices()
    }

    pub async fn set_power_profile(&self, profile: &str) -> Result<(), String> {
        power::write_power_profile(profile)
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
        wifi::wifi_status()
    }

    pub async fn connect_wifi(&self, ssid: &str, credential_ref: &str) -> Result<String, String> {
        wifi::connect_wifi(ssid, credential_ref).await
    }

    pub async fn set_default_audio(&self, name: &str) -> Result<(), String> {
        audio::set_default_device(name).await
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
    async fn connect_wifi_requires_credential_or_wpa_cli() {
        let err = KernelHandler::new()
            .connect_wifi("example", "")
            .await
            .unwrap_err();
        assert!(
            err.contains("credential_ref") || err.contains("wpa_cli") || err.contains("wireless"),
            "unexpected: {err}"
        );
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
