//! Tier B cloud router (OpenAI-compatible API).

use crate::client::{mcp_call, trace_id};
use crate::planner::PlanStep;
use serde_json::{json, Value};

pub struct CloudRouter {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl CloudRouter {
    pub fn from_env() -> Option<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .or_else(|_| std::env::var("THE_MACHINE_CLOUD_API_KEY"))
            .ok()?;
        let base_url = std::env::var("OPENAI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model = std::env::var("THE_MACHINE_CLOUD_MODEL")
            .unwrap_or_else(|_| "gpt-4o-mini".to_string());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .ok()?;
        Some(CloudRouter {
            client,
            base_url,
            api_key,
            model,
        })
    }

    pub async fn plan(
        &self,
        intent: &str,
        text: &str,
        payload: &Value,
        provenance_trace: &str,
    ) -> Option<Vec<PlanStep>> {
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
        let resp = self
            .client
            .post(format!("{}/chat/completions", self.base_url.trim_end_matches('/')))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .ok()?;
        let v: Value = resp.json().await.ok()?;
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
        if plan.is_empty() { None } else { Some(plan) }
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
        let mut calls = existing.get("calls").and_then(|v| v.as_u64()).unwrap_or(0) + 1;
        let mut total_tokens = existing.get("tokens").and_then(|v| v.as_u64()).unwrap_or(0) + tokens;
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
            json!({ "path": path, "value": { "calls": calls, "tokens": total_tokens, "traces": traces } }),
        )
        .await;
    }
}

pub fn new_trace() -> String {
    trace_id()
}
