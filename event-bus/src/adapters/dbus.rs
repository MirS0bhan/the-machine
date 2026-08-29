//! D-Bus adapter: monitors notifications and login events when dbus-monitor is available.

use super::publish_event;
use serde_json::json;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, warn};

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match spawn_monitor().await {
            Ok(()) => warn!("dbus-monitor exited; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
    }
}

async fn spawn_monitor() -> Result<(), String> {
    let mut child = tokio::process::Command::new("dbus-monitor")
        .args([
            "--system",
            "type='signal',interface='org.freedesktop.Notifications',member='Notify'",
            "type='signal',interface='org.freedesktop.login1.Manager',member='PrepareForSleep'",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;

    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut reader = BufReader::new(stdout).lines();
    info!("dbus-monitor started");
    while let Some(line) = reader.next_line().await.map_err(|e| e.to_string())? {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.contains("Notify") {
            publish_event(
                "notification",
                "desktop.notify",
                json!({ "raw": line, "summary": extract_field(line, "string") }),
            )
            .await;
        } else if line.contains("PrepareForSleep") {
            publish_event(
                "system",
                "login.prepare_sleep",
                json!({ "raw": line }),
            )
            .await;
        }
    }
    let _ = child.kill().await;
    Ok(())
}

fn extract_field(line: &str, kind: &str) -> String {
    if line.contains(kind) {
        line.chars().take(120).collect()
    } else {
        String::new()
    }
}
