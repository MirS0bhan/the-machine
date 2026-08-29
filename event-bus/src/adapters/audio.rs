//! PipeWire/audio decision events (device state, not audio buffers).

use super::publish_event;
use serde_json::json;
use std::path::Path;
use tracing::info;

pub async fn run() {
    let monitor_paths = [
        "/run/user/0/pipewire-0",
        "/run/user/1000/pipewire-0",
    ];
    let mut last_state = String::new();
    loop {
        for p in monitor_paths {
            let path = Path::new(p);
            if path.exists() {
                let state = if path.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                    "running"
                } else {
                    "present"
                };
                if state != last_state {
                    last_state = state.to_string();
                    publish_event(
                        "audio",
                        "pipewire.state",
                        json!({ "path": p, "state": state }),
                    )
                    .await;
                    info!("pipewire state: {} at {}", state, p);
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(15)).await;
    }
}
