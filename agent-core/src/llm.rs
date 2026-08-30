//! Local model integration via MCP bus.

use crate::client::mcp_call;
use crate::planner::PlanStep;
use crate::skills::{build_skill_prompt, Skill};

pub struct Classification {
    pub intent: String,
    pub confidence: f64,
    pub complexity: String,
    pub routing: String,
    pub requires_cloud: bool,
}

pub async fn classify_intent(text: &str, category: &str, skills: &[Skill]) -> Classification {
    let skill_ctx = build_skill_prompt(skills);
    if let Some(result) = mcp_call(
        "localmodel.classify_intent",
        serde_json::json!({
            "text": text,
            "category": category,
            "skill_context": skill_ctx,
        }),
    )
    .await
    {
        let intent = result
            .get("intent")
            .and_then(|v| v.as_str())
            .unwrap_or("generic")
            .to_string();
        let confidence = result
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.8);
        let complexity = result
            .get("complexity")
            .and_then(|v| v.as_str())
            .unwrap_or("medium")
            .to_string();
        let routing = result
            .get("routing")
            .and_then(|v| v.as_str())
            .unwrap_or("local")
            .to_string();
        let requires_cloud = result
            .get("requires_cloud")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        return Classification {
            intent,
            confidence,
            complexity,
            routing,
            requires_cloud,
        };
    }
    // Fallback heuristic when local-model daemon unavailable.
    let t = text.to_lowercase();
    let intent = if category == "boot" {
        "boot.greet"
    } else if category == "scheduler" {
        "heartbeat"
    } else if t.contains("calc") {
        "calculator"
    } else if t.contains("play") {
        "media_control"
    } else if t.contains("notification") {
        "notification.triage"
    } else if category == "input" || t.contains("chat") {
        "chat.message"
    } else {
        "generic"
    };
    Classification {
        intent: intent.into(),
        confidence: 0.7,
        complexity: "medium".into(),
        routing: "local".into(),
        requires_cloud: false,
    }
}

pub async fn plan_from_model(
    intent: &str,
    text: &str,
    payload: &serde_json::Value,
    skills: &[Skill],
) -> Vec<PlanStep> {
    let skill_ctx = build_skill_prompt(skills);
    let prompt = format!(
        r#"You are the Agent Core planner for The Machine OS.
Intent: {intent}
User text: {text}
Payload: {payload}
{skill_ctx}
Respond with JSON only:
{{"steps":[{{"action":"mcp.method","params":{{}}}}]}}
Use actions: lambda.register, lambda.invoke, ui.patch, state.patch, state.set, event.publish."#
    );
    if let Some(result) = mcp_call(
        "localmodel.complete",
        serde_json::json!({
            "prompt": prompt,
            "max_tokens": 1024,
            "temperature": 0.2,
        }),
    )
    .await
    {
        let text = result.get("text").and_then(|v| v.as_str()).unwrap_or("");
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(steps) = parsed.get("steps").and_then(|s| s.as_array()) {
                let mut plan = Vec::new();
                for step in steps {
                    if let (Some(action), params) = (
                        step.get("action").and_then(|v| v.as_str()),
                        step.get("params")
                            .cloned()
                            .unwrap_or(serde_json::Value::Null),
                    ) {
                        plan.push(PlanStep {
                            action: action.to_string(),
                            params,
                        });
                    }
                }
                if !plan.is_empty() {
                    return plan;
                }
            }
        }
    }
    crate::planner::build_plan_heuristic(intent, payload, text)
}
