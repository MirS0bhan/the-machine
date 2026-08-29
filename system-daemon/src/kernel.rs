//! Kernel/hardware abstraction: power, display, network, audio.
//!
//! Read paths prefer live sysfs/proc data (G14) and fall back to the
//! previous mock values when the host has no corresponding interface
//! (CI containers, nographic QEMU). Mutations write sysfs when the
//! node is writable and otherwise return a structured error instead of
//! silently succeeding.

use common::{AudioDevice, DisplayMode, NetworkInterface};
use std::path::Path;

const LOOPBACK_ARPHRD: &str = "772";

pub struct KernelHandler;

impl KernelHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn get_power_profile(&self) -> String {
        if let Some(raw) = read_trimmed("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor") {
            return map_governor(&raw);
        }
        if let Some(raw) = read_trimmed("/sys/firmware/acpi/pm_profile") {
            return map_acpi_profile(&raw);
        }
        "balanced".to_string()
    }

    pub fn get_display_modes(&self) -> Vec<DisplayMode> {
        let mut modes = drm_modes();
        if modes.is_empty() {
            modes.extend(framebuffer_mode());
        }
        if modes.is_empty() {
            modes.push(DisplayMode {
                width: 1920,
                height: 1080,
                refresh: 60.0,
                current: true,
            });
        }
        modes
    }

    pub fn list_interfaces(&self) -> Vec<NetworkInterface> {
        let parsed = collect_net_interfaces("/sys/class/net");
        if parsed.is_empty() {
            vec![NetworkInterface {
                name: "lo".to_string(),
                r#type: "loopback".to_string(),
                state: "up".to_string(),
            }]
        } else {
            parsed
        }
    }

    pub fn list_audio_devices(&self) -> Vec<AudioDevice> {
        let cards = std::fs::read_to_string("/proc/asound/cards").unwrap_or_default();
        let mut devices = parse_asound_cards(&cards);
        if devices.is_empty() {
            devices.push(AudioDevice {
                name: "default".to_string(),
                r#type: "output".to_string(),
                default: true,
            });
        }
        devices
    }

    pub async fn set_power_profile(&self, profile: &str) -> Result<(), String> {
        let gov = match profile {
            "performance" => "performance",
            "powersave" => "powersave",
            "balanced" => "schedutil",
            other => return Err(format!("unknown power profile: {other}")),
        };
        let path = "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor";
        if !Path::new(path).exists() {
            return Err("cpufreq governor not available on this host".into());
        }
        std::fs::write(path, gov).map_err(|e| format!("cannot set governor: {e}"))
    }

    pub async fn set_display_mode(
        &self,
        _width: u32,
        _height: u32,
        _refresh: f32,
    ) -> Result<(), String> {
        Err("display mode changes require a DRM master; use compositor.set_mode".into())
    }

    pub async fn set_interface_state(&self, name: &str, state: &str) -> Result<(), String> {
        if !matches!(state, "up" | "down") {
            return Err(format!("invalid interface state: {state}"));
        }
        let path = format!("/sys/class/net/{name}");
        if !Path::new(&path).exists() {
            return Err(format!("unknown interface: {name}"));
        }
        // `flags` is not a portable write interface; refuse rather than fake success.
        Err(format!(
            "net.set_interface_state({name}={state}) requires CAP_NET_ADMIN via ip/netlink"
        ))
    }

    pub async fn connect_wifi(&self, _ssid: &str, _credential_ref: &str) -> Result<String, String> {
        Err("wifi connect is not wired; credential-ref path is reserved".into())
    }

    pub async fn set_default_audio(&self, name: &str) -> Result<(), String> {
        Err(format!(
            "audio.set_default({name}) requires a PipeWire/ALSA session"
        ))
    }
}

fn read_trimmed(path: &str) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn map_governor(raw: &str) -> String {
    match raw.trim() {
        "performance" => "performance".into(),
        "powersave" | "conservative" => "powersave".into(),
        "schedutil" | "ondemand" | "userspace" => "balanced".into(),
        other => other.to_string(),
    }
}

pub fn map_acpi_profile(raw: &str) -> String {
    // ACPI pm_profile is a small integer; see include/acpi/actbl.h.
    match raw.trim() {
        "0" | "1" => "balanced".into(),
        "2" => "performance".into(),
        "3" | "4" | "5" => "powersave".into(),
        other => other.to_string(),
    }
}

pub fn parse_asound_cards(text: &str) -> Vec<AudioDevice> {
    let mut devices = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || !line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            continue;
        }
        // " 0 [PCH            ]: HDA-Intel - HDA Intel PCH"
        let name = line
            .split_once(':')
            .map(|(_, rest)| rest.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| line.to_string());
        let default = devices.is_empty();
        let r#type = if name.to_ascii_lowercase().contains("usb") {
            "usb"
        } else {
            "output"
        };
        devices.push(AudioDevice {
            name,
            r#type: r#type.to_string(),
            default,
        });
    }
    devices
}

pub fn collect_net_interfaces(sys_class_net: &str) -> Vec<NetworkInterface> {
    let Ok(entries) = std::fs::read_dir(sys_class_net) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.is_empty() {
            continue;
        }
        let base = entry.path();
        let state = std::fs::read_to_string(base.join("operstate"))
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".into());
        let arphrd = std::fs::read_to_string(base.join("type"))
            .ok()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let r#type = if arphrd == LOOPBACK_ARPHRD {
            "loopback"
        } else if base.join("wireless").exists() {
            "wifi"
        } else {
            "ethernet"
        };
        out.push(NetworkInterface {
            name,
            r#type: r#type.to_string(),
            state,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn drm_modes() -> Vec<DisplayMode> {
    let Ok(entries) = std::fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };
    let mut modes = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.contains("card") || !name.contains('-') {
            continue;
        }
        let status = std::fs::read_to_string(entry.path().join("status"))
            .ok()
            .map(|s| s.trim().to_string());
        if status.as_deref() != Some("connected") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(entry.path().join("modes")) else {
            continue;
        };
        modes.extend(parse_drm_modes(&text));
        if !modes.is_empty() {
            break;
        }
    }
    modes
}

pub fn parse_drm_modes(text: &str) -> Vec<DisplayMode> {
    let mut modes = Vec::new();
    for line in text.lines() {
        if let Some((w, h)) = parse_mode_line(line) {
            let current = modes.is_empty();
            modes.push(DisplayMode {
                width: w,
                height: h,
                refresh: 60.0,
                current,
            });
        }
    }
    modes
}

fn parse_mode_line(line: &str) -> Option<(u32, u32)> {
    let line = line.trim();
    let (wh, _) = line
        .split_once(|c: char| c == '@' || c.is_ascii_whitespace())
        .unwrap_or((line, ""));
    let (w, h) = wh.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

fn framebuffer_mode() -> Vec<DisplayMode> {
    let Some(raw) = read_trimmed("/sys/class/graphics/fb0/virtual_size") else {
        return Vec::new();
    };
    let Some((w, h)) = raw.split_once(',') else {
        return Vec::new();
    };
    let (Ok(width), Ok(height)) = (w.trim().parse(), h.trim().parse()) else {
        return Vec::new();
    };
    vec![DisplayMode {
        width,
        height,
        refresh: 60.0,
        current: true,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governor_mapping() {
        assert_eq!(map_governor("performance"), "performance");
        assert_eq!(map_governor("powersave"), "powersave");
        assert_eq!(map_governor("schedutil"), "balanced");
    }

    #[test]
    fn acpi_profile_mapping() {
        assert_eq!(map_acpi_profile("2"), "performance");
        assert_eq!(map_acpi_profile("1"), "balanced");
        assert_eq!(map_acpi_profile("5"), "powersave");
    }

    #[test]
    fn asound_parser() {
        let sample = "\
 0 [PCH            ]: HDA-Intel - HDA Intel PCH
                      HDA Intel PCH at 0x...
 1 [USB            ]: USB-Audio - USB Headset
";
        let devices = parse_asound_cards(sample);
        assert_eq!(devices.len(), 2);
        assert!(devices[0].default);
        assert_eq!(devices[1].r#type, "usb");
    }

    #[test]
    fn drm_mode_parser() {
        let modes = parse_drm_modes("1920x1080\n1280x720@60\n");
        assert_eq!(modes.len(), 2);
        assert_eq!(modes[0].width, 1920);
        assert!(modes[0].current);
        assert_eq!(modes[1].height, 720);
    }

    #[test]
    fn net_interfaces_from_sysfs() {
        // Every Linux CI image has loopback.
        let ifaces = collect_net_interfaces("/sys/class/net");
        assert!(
            ifaces
                .iter()
                .any(|i| i.name == "lo" && i.r#type == "loopback"),
            "expected loopback in {ifaces:?}"
        );
    }
}
