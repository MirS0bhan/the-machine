//! Tier B cloud router (OpenAI-compatible API).

use crate::client::{mcp_call, trace_id};
use crate::planner::PlanStep;
use crate::secrets::{cloud_key_status, load_cloud_api_key};
use serde_json::{json, Value};

pub struct CloudRouter {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
    key_source: String,
}

impl CloudRouter {
    pub fn from_env() -> Option<Self> {
        let secret = load_cloud_api_key()?;
        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model =
            std::env::var("THE_MACHINE_CLOUD_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .ok()?;
        Some(CloudRouter {
            client,
            base_url,
            api_key: secret.api_key,
            model,
            key_source: secret.source,
        })
    }

    pub fn key_source(&self) -> &str {
        &self.key_source
    }

    pub async fn plan(
        &self,
        intent: &str,
        text: &str,
        payload: &Value,
        provenance_trace: &str,
    ) -> Option<Vec<PlanStep>> {
        if !cloud_policy_allowed("cloud.plan").await {
            tracing::warn!("cloud plan blocked by policy broker");
            return None;
        }
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": "You are Agent Core Tier B. Return JSON: {\"steps\":[{\"action\":\"...\",\"params\":{}}]}"},
                {"role": "user", "content": format!(
                    "trace_id={}\nintent={}\ntext={}\npayload={}",
                    provenance_trace, intent, text, payload
                )}
            ],
            "temperature": 0.2,
        });
        let v = self.chat_completions(&body).await?;
        let content = v
            .get("choices")?
            .as_array()?
            .first()?
            .get("message")?
            .get("content")?
            .as_str()?;
        self.record_usage(provenance_trace, &v).await;
        let parsed: Value = serde_json::from_str(content).ok()?;
        let mut plan = Vec::new();
        for step in parsed.get("steps")?.as_array()? {
            plan.push(PlanStep {
                action: step.get("action")?.as_str()?.to_string(),
                params: step.get("params").cloned().unwrap_or(Value::Null),
            });
        }
        if plan.is_empty() {
            None
        } else {
            Some(plan)
        }
    }

    /// Conversational completion for chat UI replies (plain text, not plan JSON).
    pub async fn complete_chat(&self, user_text: &str, provenance_trace: &str) -> Option<String> {
        if !cloud_policy_allowed("cloud.complete").await {
            tracing::warn!("cloud chat blocked by policy broker");
            return None;
        }
        let body = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are The Machine, a helpful on-device OS assistant. Reply briefly in plain text. Do not return JSON or markdown fences."
                },
                {"role": "user", "content": user_text}
            ],
            "temperature": 0.5,
            "max_tokens": 512,
        });
        let v = self.chat_completions(&body).await?;
        let content = v
            .get("choices")?
            .as_array()?
            .first()?
            .get("message")?
            .get("content")?
            .as_str()?
            .trim();
        if content.is_empty() {
            return None;
        }
        self.record_usage(provenance_trace, &v).await;
        Some(content.to_string())
    }

    async fn chat_completions(&self, body: &Value) -> Option<Value> {
        let resp = self
            .client
            .post(format!(
                "{}/chat/completions",
                self.base_url.trim_end_matches('/')
            ))
            .bearer_auth(&self.api_key)
            .json(body)
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            tracing::warn!("cloud API returned {}", resp.status());
            return None;
        }
        resp.json().await.ok()
    }

    async fn record_usage(&self, trace_id: &str, response: &Value) {
        let tokens = response
            .get("usage")
            .and_then(|u| u.get("total_tokens"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);
        let path = format!("task.cloud_usage.{}", chrono::Utc::now().format("%Y%m%d"));
        let existing = mcp_call("state.get", json!({ "path": path }))
            .await
            .and_then(|v| v.get("value").cloned())
            .unwrap_or(json!({ "calls": 0, "tokens": 0, "traces": [] }));
        let calls = existing.get("calls").and_then(|v| v.as_u64()).unwrap_or(0) + 1;
        let total_tokens = existing.get("tokens").and_then(|v| v.as_u64()).unwrap_or(0) + tokens;
        let mut traces = existing
            .get("traces")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        traces.push(json!({ "trace_id": trace_id, "tokens": tokens, "at": chrono::Utc::now().to_rfc3339() }));
        if traces.len() > 50 {
            traces.drain(0..traces.len() - 50);
        }
        let _ = mcp_call(
            "state.set",
            json!({ "path": path, "value": { "calls": calls, "tokens": total_tokens, "traces": traces, "key_source": self.key_source } }),
        )
        .await;
    }
}

pub fn new_trace() -> String {
    trace_id()
}

pub fn status() -> Value {
    cloud_key_status()
}

async fn cloud_policy_allowed(action: &str) -> bool {
    mcp_call(
        "policy.check",
        json!({
            "capability": "CAP_CLOUD_INFERENCE",
            "principal": "agent-core",
            "method": action,
            "path": action,
            "request": { "principal": "agent-core", "action": action },
        }),
    )
    .await
    .and_then(|v| {
        v.get("decision")
            .and_then(|d| d.as_str().map(|s| s == "ALLOW"))
    })
    // Broker unreachable: fail closed for cloud (local/heuristic still available).
    .unwrap_or(false)
}
