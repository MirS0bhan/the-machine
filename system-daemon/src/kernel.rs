//! Kernel/hardware abstraction: power, display, network, audio.
//!
//! These are placeholder stubs returning mock data until the real
//! kernel interfaces are wired up.

use common::{AudioDevice, DisplayMode, NetworkInterface};

pub struct KernelHandler;

impl KernelHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn get_power_profile(&self) -> String {
        "balanced".to_string()
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

    pub async fn set_power_profile(&self, _profile: &str) -> Result<(), String> {
        Ok(())
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

    pub async fn connect_wifi(
        &self,
        _ssid: &str,
        _credential_ref: &str,
    ) -> Result<(), String> {
        Ok(())
    }

    pub async fn set_default_audio(&self, _name: &str) -> Result<(), String> {
        Ok(())
    }
}
