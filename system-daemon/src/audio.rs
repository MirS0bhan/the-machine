//! Audio devices via PipeWire / PulseAudio (`pactl`).

use common::AudioDevice;
use std::process::Command;

fn pactl_sync(args: &[&str]) -> Result<String, String> {
    let output = Command::new("pactl")
        .args(args)
        .output()
        .map_err(|e| format!("pactl not available: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("pactl failed: {stderr}"));
    }
    Ok(stdout)
}

async fn pactl(args: &[&str]) -> Result<String, String> {
    pactl_sync(args)
}

/// List sinks and sources from `pactl list short`.
pub fn list_audio_devices() -> Vec<AudioDevice> {
    let Ok(sinks) = pactl_sync(&["list", "short", "sinks"]) else {
        return fallback_devices();
    };
    let Ok(sources) = pactl_sync(&["list", "short", "sources"]) else {
        return fallback_devices();
    };

    let default_sink = pactl_sync(&["get-default-sink"])
        .unwrap_or_default()
        .trim()
        .to_string();

    let mut devices = Vec::new();
    for line in sinks.lines() {
        let mut parts = line.split_whitespace();
        let _index = parts.next();
        let name = parts.next().unwrap_or("unknown").to_string();
        devices.push(AudioDevice {
            name: name.clone(),
            r#type: "output".into(),
            default: name == default_sink,
        });
    }
    for line in sources.lines() {
        let mut parts = line.split_whitespace();
        let _index = parts.next();
        let name = parts.next().unwrap_or("unknown").to_string();
        if name.ends_with(".monitor") {
            continue;
        }
        devices.push(AudioDevice {
            name,
            r#type: "input".into(),
            default: false,
        });
    }
    if devices.is_empty() {
        return fallback_devices();
    }
    devices
}

fn fallback_devices() -> Vec<AudioDevice> {
    vec![AudioDevice {
        name: "default".into(),
        r#type: "output".into(),
        default: true,
    }]
}

/// Set default sink via `pactl set-default-sink`.
pub async fn set_default_device(name: &str) -> Result<(), String> {
    pactl(&["set-default-sink", name]).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_returns_at_least_fallback() {
        let devices = list_audio_devices();
        assert!(!devices.is_empty());
    }
}
