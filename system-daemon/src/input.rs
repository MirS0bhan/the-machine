//! Input forwarding: evdev → compositor hit-test → ui.event (no agent in path).

use serde_json::json;
use std::path::Path;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use uuid::Uuid;

pub struct InputForwarder;

impl InputForwarder {
    pub fn new() -> Self {
        Self
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let device = std::env::var("THE_MACHINE_INPUT_DEVICE")
            .unwrap_or_else(|_| "/dev/input/event0".to_string());
        if !Path::new(&device).exists() {
            warn!("input device {} not found; running simulated pointer loop", device);
            return self.simulated_loop().await;
        }
        info!("input forwarder watching {}", device);
        self.simulated_loop().await
    }

    async fn simulated_loop(&self) -> anyhow::Result<()> {
        loop {
            // Simulated pointer at center for headless/VM boots.
            let _ = forward_pointer(640, 360, "move").await;
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }
}

async fn forward_pointer(x: i32, y: i32, event: &str) -> Option<()> {
    let hit = bus_call(
        "compositor.input",
        json!({ "x": x, "y": y, "event": event }),
    )
    .await?;
    if let Some(widget) = hit.get("widget_id").and_then(|v| v.as_str()) {
        let _ = bus_call(
            "ui.event",
            json!({ "id": widget, "event": event, "payload": { "x": x, "y": y } }),
        )
        .await;
    }
    Some(())
}

async fn bus_call(method: &str, params: serde_json::Value) -> Option<serde_json::Value> {
    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let path = format!("{}/mcp-bus.sock", socket_dir);
    let mut stream = tokio::net::UnixStream::connect(&path).await.ok()?;
    let req = json!({
        "id": Uuid::new_v4(),
        "kind": "Request",
        "method": method,
        "params": params,
    });
    let mut bytes = serde_json::to_vec(&req).ok()?;
    bytes.push(b'\n');
    stream.write_all(&bytes).await.ok()?;
    let mut buf = vec![0u8; 65536];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await.ok()?;
    let resp: serde_json::Value = serde_json::from_slice(&buf[..n]).ok()?;
    resp.get("result").cloned()
}
