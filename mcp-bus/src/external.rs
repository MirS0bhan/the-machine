//! Allow-listed external MCP server proxy handlers.

use dashmap::DashMap;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone)]
pub struct ExternalServer {
    pub id: String,
    pub base_url: String,
    pub allowed_methods: Vec<String>,
}

pub struct ExternalRegistry {
    servers: DashMap<String, ExternalServer>,
}

impl ExternalRegistry {
    pub fn new() -> Self {
        ExternalRegistry {
            servers: DashMap::new(),
        }
    }

    pub fn register(&self, id: &str, base_url: &str, methods: Vec<String>) -> Value {
        self.servers.insert(
            id.to_string(),
            ExternalServer {
                id: id.to_string(),
                base_url: base_url.to_string(),
                allowed_methods: methods,
            },
        );
        json!({ "registered": id, "base_url": base_url })
    }

    pub fn list(&self) -> Value {
        let items: Vec<Value> = self
            .servers
            .iter()
            .map(|e| {
                json!({
                    "id": e.id,
                    "base_url": e.base_url,
                    "allowed_methods": e.allowed_methods,
                })
            })
            .collect();
        json!({ "servers": items })
    }

    pub async fn forward(&self, server_id: &str, method: &str, params: Value) -> Option<Value> {
        let server = self.servers.get(server_id)?;
        if !server.allowed_methods.is_empty() && !server.allowed_methods.iter().any(|m| m == method || m == "*") {
            return Some(json!({ "error": "method not allowed on external server" }));
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .ok()?;
        let url = format!("{}/mcp/{}", server.base_url.trim_end_matches('/'), method);
        let resp = client.post(&url).json(&params).send().await.ok()?;
        resp.json().await.ok()
    }
}

pub type SharedExternal = Arc<ExternalRegistry>;
