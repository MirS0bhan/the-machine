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
    if category == "boot" {
        return Classification {
            intent: "boot.greet".into(),
            confidence: 1.0,
            complexity: "low".into(),
            routing: "local".into(),
            requires_cloud: false,
        };
    }
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
        "boot.greet".to_string()
    } else if category == "scheduler" {
        "heartbeat".to_string()
    } else if t.contains("calc") {
        "calculator".to_string()
    } else if t.contains("play") {
        "media_control".to_string()
    } else if t.contains("notification") {
        "notification.triage".to_string()
    } else if let Some(desktop) = crate::planner::desktop_intent_from_text(text) {
        desktop.to_string()
    } else if category == "input" || t.contains("chat") {
        "chat.message".to_string()
    } else {
        "generic".to_string()
    };
    Classification {
        intent,
        confidence: 0.7,
        complexity: "medium".into(),
        routing: "local".into(),
        requires_cloud: false,
    }
}

/// Plain-text chat reply via local model (not a plan JSON).
pub async fn complete_chat(user_text: &str) -> Option<String> {
    let prompt = format!(
        "You are The Machine, a helpful on-device OS assistant. Reply briefly in plain text.\n\nUser: {user_text}\nAssistant:"
    );
    let result = mcp_call(
        "localmodel.complete",
        serde_json::json!({
            "prompt": prompt,
            "max_tokens": 512,
            "temperature": 0.5,
        }),
    )
    .await?;
    let text = result
        .get("text")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())?;
    Some(text.to_string())
}

pub async fn plan_from_model(
    intent: &str,
    text: &str,
    payload: &serde_json::Value,
    skills: &[Skill],
) -> Vec<PlanStep> {
    let skill_ctx = build_skill_prompt(skills);
    let prompt = format!(
        r#"You are the Agent Core planner for The Machine agentic desktop.
Intent: {intent}
User text: {text}
Payload: {payload}
{skill_ctx}
Respond with JSON only:
{{"steps":[{{"action":"mcp.method","params":{{}}}}]}}
Use actions: lambda.register, lambda.invoke, ui.patch, state.patch, state.set, event.publish, agent.status.
Prefer inserting agent UI under anchor "ui.workspace" (button/list/dialog with mcp bindings).
Twelve AUIL primitives only. Never invent Confirmation Surfaces — those are broker-owned."#
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
                let plan = sanitize_plan(plan);
                if !plan.is_empty() {
                    return plan;
                }
            }
        }
    }
    crate::planner::build_plan_heuristic(intent, payload, text)
}

/// Drop model-emitted steps the boot path cannot honour, fail-closed.
///
/// A model is free to hallucinate a thirteenth primitive or a Confirmation
/// Surface; neither may reach `ui.patch`. Steps whose `ui.patch` ops reference
/// an unknown node kind are removed rather than "best-effort" painted.
pub fn sanitize_plan(plan: Vec<PlanStep>) -> Vec<PlanStep> {
    plan.into_iter()
        .filter_map(|mut step| {
            if step.action != "ui.patch" {
                // Confirmation Surfaces are broker-owned; a plan may not forge one.
                if step.action.starts_with("policy.confirm")
                    || step.action == "compositor.confirmation.set_active"
                {
                    return None;
                }
                return Some(step);
            }
            let ops = step
                .params
                .get("ops")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let kept: Vec<serde_json::Value> = ops
                .into_iter()
                .filter(|op| {
                    match op.get("node").and_then(|n| n.get("type")).and_then(|v| v.as_str()) {
                        Some(kind) => {
                            crate::desktop::PRIMITIVES.contains(&kind) || kind == "container"
                        }
                        // update / remove / move ops carry no node type.
                        None => true,
                    }
                })
                .collect();
            if kept.is_empty() {
                return None;
            }
            step.params = serde_json::json!({ "ops": kept });
            Some(step)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patch(kind: &str) -> PlanStep {
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": { "id": "ui.x", "type": kind, "props": {} }
                }]
            }),
        }
    }

    #[test]
    fn sanitize_keeps_every_primitive() {
        for kind in crate::desktop::PRIMITIVES {
            assert_eq!(sanitize_plan(vec![patch(kind)]).len(), 1, "kind {kind}");
        }
    }

    #[test]
    fn sanitize_drops_invented_primitive() {
        assert!(sanitize_plan(vec![patch("carousel")]).is_empty());
    }

    #[test]
    fn sanitize_keeps_update_ops_without_node_type() {
        let step = PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{ "op": "update", "id": "ui.activity", "props": { "text": "hi" } }]
            }),
        };
        assert_eq!(sanitize_plan(vec![step]).len(), 1);
    }

    #[test]
    fn sanitize_refuses_forged_confirmation_surface() {
        let step = PlanStep {
            action: "compositor.confirmation.set_active".into(),
            params: serde_json::json!({ "active": true }),
        };
        assert!(sanitize_plan(vec![step]).is_empty());
    }
}
