//! Rich heartbeat aggregation for proactive agent wakes.

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

pub async fn gather_rich_snapshot() -> Value {
    let environment = basic_environment().await;
    let lambdas = query_bus("lambda.health", json!({})).await;
    let ui = query_bus("ui.status", json!({})).await;
    let policy = query_bus("policy.audit", json!({ "limit": 5 })).await;
    let system = query_bus("system-daemon.stats", json!({})).await;
    let routes = query_bus("bus.list_routes", json!({})).await;

    let policy_holds = policy
        .as_ref()
        .and_then(|p| p.get("pending_confirmations"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    json!({
        "timestamp_ms": chrono::Utc::now().timestamp_millis(),
        "environment": environment,
        "lambdas": lambdas.unwrap_or(json!({})),
        "ui": ui.unwrap_or(json!({})),
        "policy": {
            "hold_queue_depth": policy_holds,
            "recent_audit": policy,
        },
        "system": system.unwrap_or(json!({})),
        "registry": {
            "route_count": routes
                .as_ref()
                .and_then(|r| r.get("routes"))
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0),
        },
    })
}

async fn basic_environment() -> Value {
    let uptime = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(|v| v.to_string()))
        .unwrap_or_else(|| "0".into());
    json!({
        "uptime_secs": uptime,
        "hostname": std::env::var("HOSTNAME").unwrap_or_else(|_| "the-machine".into()),
    })
}

async fn query_bus(method: &str, params: Value) -> Option<Value> {
    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let path = format!("{}/mcp-bus.sock", socket_dir);
    let stream = tokio::net::UnixStream::connect(&path).await.ok()?;
    let (mut reader, mut writer) = stream.into_split();
    let req = json!({
        "id": Uuid::new_v4(),
        "kind": "Request",
        "method": method,
        "params": params,
    });
    let mut bytes = serde_json::to_vec(&req).ok()?;
    bytes.push(b'\n');
    writer.write_all(&bytes).await.ok()?;
    writer.flush().await.ok()?;
    let mut buf = vec![0u8; 65536];
    let n = reader.read(&mut buf).await.ok()?;
    if n == 0 {
        return None;
    }
    let resp: Value = serde_json::from_slice(&buf[..n]).ok()?;
    resp.get("result").cloned()
}
