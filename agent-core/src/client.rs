//! Shared MCP bus client for agent-core.

use common::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

pub async fn mcp_call(method: &str, params: serde_json::Value) -> Option<serde_json::Value> {
    let socket_dir = std::env::var("THE_MACHINE_SOCKET_DIR")
        .unwrap_or_else(|_| "/run/the-machine".to_string());
    let path = format!("{}/mcp-bus.sock", socket_dir);
    let stream = tokio::net::UnixStream::connect(&path).await.ok()?;
    let (mut reader, mut writer) = stream.into_split();
    let req = McpMessage::request(Uuid::new_v4(), method, Some(params));
    let mut bytes = serde_json::to_vec(&req).ok()?;
    bytes.push(b'\n');
    writer.write_all(&bytes).await.ok()?;
    writer.flush().await.ok()?;
    let mut buf = vec![0u8; 65536];
    let n = reader.read(&mut buf).await.ok()?;
    if n == 0 {
        return None;
    }
    let resp: serde_json::Value = serde_json::from_slice(&buf[..n]).ok()?;
    resp.get("result").cloned()
}

pub fn trace_id() -> String {
    Uuid::new_v4().to_string()
}
