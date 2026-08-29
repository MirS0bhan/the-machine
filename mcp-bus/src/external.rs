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

    pub fn register(
        &self,
        id: &str,
        base_url: &str,
        methods: Vec<String>,
    ) -> Result<Value, String> {
        if id.is_empty() {
            return Err("id required".into());
        }
        validate_external_endpoint(base_url, &methods)?;
        self.servers.insert(
            id.to_string(),
            ExternalServer {
                id: id.to_string(),
                base_url: base_url.to_string(),
                allowed_methods: methods,
            },
        );
        Ok(json!({ "registered": id, "base_url": base_url }))
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
        if !server.allowed_methods.iter().any(|m| m == method) {
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

/// External MCP endpoints must be HTTPS (or HTTP on loopback) with an
/// explicit method allow-list. Wildcards and link-local metadata hosts
/// are rejected so `bus.external.forward` cannot be used as an open proxy.
pub fn validate_external_endpoint(base_url: &str, methods: &[String]) -> Result<(), String> {
    if base_url.is_empty() {
        return Err("base_url required".into());
    }
    if methods.is_empty() {
        return Err("allowed_methods required".into());
    }
    if methods.iter().any(|m| m == "*") {
        return Err("wildcard allowed_methods is not permitted".into());
    }
    let lower = base_url.to_ascii_lowercase();
    let https = lower.starts_with("https://");
    let loopback_http = lower.starts_with("http://127.0.0.1")
        || lower.starts_with("http://localhost")
        || lower.starts_with("http://[::1]");
    if !https && !loopback_http {
        return Err("base_url must be https:// or http://localhost".into());
    }
    for blocked in [
        "169.254.169.254",
        "metadata.google.internal",
        "metadata.google",
    ] {
        if lower.contains(blocked) {
            return Err("base_url host is not allowed".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_open_proxy_defaults() {
        assert!(validate_external_endpoint("https://evil.example/mcp", &["*".into()]).is_err());
        assert!(validate_external_endpoint("http://169.254.169.254/", &["foo".into()]).is_err());
        assert!(validate_external_endpoint("file:///etc/passwd", &["foo".into()]).is_err());
        assert!(validate_external_endpoint("gopher://x", &["foo".into()]).is_err());
        assert!(validate_external_endpoint("", &["foo".into()]).is_err());
    }

    #[test]
    fn accepts_https_and_loopback_with_explicit_methods() {
        assert!(
            validate_external_endpoint("https://tools.example.com/mcp", &["search".into()]).is_ok()
        );
        assert!(validate_external_endpoint("http://127.0.0.1:8080", &["ping".into()]).is_ok());
        assert!(validate_external_endpoint("http://localhost:9", &["ping".into()]).is_ok());
    }

    #[test]
    fn register_requires_id_and_valid_url() {
        let reg = ExternalRegistry::new();
        assert!(reg
            .register("", "https://x.example", vec!["a".into()])
            .is_err());
        assert!(reg
            .register("ext", "http://8.8.8.8/mcp", vec!["a".into()])
            .is_err());
        assert!(reg
            .register("ext", "https://x.example", vec!["search".into()])
            .is_ok());
    }
}
