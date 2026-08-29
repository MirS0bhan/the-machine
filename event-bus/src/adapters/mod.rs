pub mod audio;
pub mod dbus;
pub mod inotify;

use serde_json::json;
use tracing::info;

pub async fn start_all() {
    info!("starting event adapters");
    tokio::spawn(dbus::run());
    tokio::spawn(inotify::run());
    tokio::spawn(audio::run());
}

pub async fn publish_event(category: &str, pattern: &str, payload: serde_json::Value) {
    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let path = format!("{}/event-bus.sock", socket_dir);
    if let Ok(mut stream) = tokio::net::UnixStream::connect(&path).await {
        let req = json!({
            "id": 1,
            "kind": "Request",
            "method": "event.publish",
            "params": {
                "category": category,
                "pattern": pattern,
                "source": "adapter",
                "payload": payload,
            }
        });
        let mut bytes = serde_json::to_vec(&req).unwrap_or_default();
        bytes.push(b'\n');
        let _ = stream.write_all(&bytes).await;
    }
}

use tokio::io::AsyncWriteExt;
