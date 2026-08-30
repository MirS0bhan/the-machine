//! Plan construction: heuristic fallback + synthesis helpers.

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub action: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

pub fn build_plan_heuristic(
    intent: &str,
    payload: &serde_json::Value,
    text: &str,
) -> Vec<PlanStep> {
    match intent {
        "boot.greet" => boot_greet_plan(),
        "heartbeat" => vec![PlanStep {
            action: "state.patch".into(),
            params: serde_json::json!({
                "ops": [{ "path": "system.last_heartbeat", "value": chrono::Utc::now().to_rfc3339() }]
            }),
        }],
        "notification.triage" => vec![
            PlanStep {
                action: "ui.patch".into(),
                params: serde_json::json!({
                    "ops": [{
                        "op": "insert",
                        "anchor": "ui.root",
                        "node": {
                            "id": "ui.notification_card",
                            "type": "card",
                            "props": { "title": "Notification", "body": payload },
                        }
                    }]
                }),
            },
            PlanStep {
                action: "state.patch".into(),
                params: serde_json::json!({
                    "ops": [{ "path": "task.last_notification", "value": payload }]
                }),
            },
        ],
        "media_control" => vec![PlanStep {
            action: "lambda.invoke".into(),
            params: serde_json::json!({
                "name": "media_player",
                "payload": { "command": "play", "query": payload.get("text").cloned().unwrap_or(serde_json::Value::Null) }
            }),
        }],
        "calculator" | "calc.eval" | "synthesize" => synthesize_lambda_plan(text, payload),
        "filesystem" => vec![PlanStep {
            action: "event.publish".into(),
            params: serde_json::json!({
                "category": "filesystem",
                "pattern": "change.detected",
                "payload": payload,
            }),
        }],
        "query" => vec![PlanStep {
            action: "state.set".into(),
            params: serde_json::json!({ "path": "task.last_query", "value": payload }),
        }],
        "chat.message" => chat_message_plan(text),
        _ => vec![PlanStep {
            action: "state.set".into(),
            params: serde_json::json!({ "path": "task.last_intent", "value": intent }),
        }],
    }
}

fn synthesize_lambda_plan(text: &str, payload: &serde_json::Value) -> Vec<PlanStep> {
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("calc.eval")
        .to_string();
    let widget_id = format!("ui.{}", name.replace('.', "_"));
    let source = r#"#!/usr/bin/env python3
import json, sys
data = json.loads(sys.stdin.read() or '{}')
expr = data.get('expression') or data.get('query') or '1+1'
try:
    result = eval(expr, {"__builtins__": {}})
except Exception as e:
    result = str(e)
print(json.dumps({"result": result}))
"#
    .to_string();
    vec![
        PlanStep {
            action: "lambda.register".into(),
            params: serde_json::json!({
                "manifest": {
                    "name": name,
                    "description": format!("Synthesized handler for: {}", text),
                    "source": source,
                    "language": "python",
                    "entrypoint": "",
                    "capabilities": [],
                    "exposes_mcp": [format!("{}.*", name.split('.').next().unwrap_or("app"))],
                }
            }),
        },
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.root",
                    "node": {
                        "id": widget_id,
                        "type": "button",
                        "props": { "label": text },
                        "bindings": [{ "type": "mcp", "target": format!("{}.run", name) }]
                    }
                }]
            }),
        },
        PlanStep {
            action: "state.patch".into(),
            params: serde_json::json!({
                "ops": [{ "path": format!("task.lambdas.{}", name), "value": { "status": "registered" } }]
            }),
        },
    ]
}

fn boot_greet_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [
                    {
                        "op": "update",
                        "id": "ui.greeting",
                        "props": { "text": "Hello! I'm The Machine." }
                    },
                    {
                        "op": "update",
                        "id": "ui.chat_log",
                        "props": { "text": "Assistant: Welcome aboard. This is your local LLM chat — type a message below and press Send." }
                    }
                ]
            }),
        },
        PlanStep {
            action: "state.set".into(),
            params: serde_json::json!({
                "path": "ui.boot_greeted",
                "value": true
            }),
        },
    ]
}

fn chat_message_plan(text: &str) -> Vec<PlanStep> {
    let user_line = format!("You: {text}\nAssistant: I received your message locally. LLM reply wiring is next.");
    vec![PlanStep {
        action: "ui.patch".into(),
        params: serde_json::json!({
            "ops": [{
                "op": "update",
                "id": "ui.chat_log",
                "props": { "text": user_line }
            }]
        }),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_plan_patches_state() {
        let plan = build_plan_heuristic("heartbeat", &serde_json::json!({}), "");
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].action, "state.patch");
    }

    #[test]
    fn calculator_plan_synthesizes_lambda() {
        let plan = build_plan_heuristic("calculator", &serde_json::json!({}), "calc 2+2");
        assert!(plan.iter().any(|s| s.action == "lambda.register"));
        assert!(plan.iter().any(|s| s.action == "ui.patch"));
    }

    #[test]
    fn boot_greet_plan_updates_chat_ui() {
        let plan = build_plan_heuristic("boot.greet", &serde_json::json!({}), "");
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].action, "ui.patch");
        let ops = plan[0].params.get("ops").and_then(|v| v.as_array()).unwrap();
        assert!(ops.iter().any(|op| {
            op.get("id").and_then(|v| v.as_str()) == Some("ui.chat_log")
        }));
    }

    #[test]
    fn chat_message_plan_appends_user_line() {
        let plan = build_plan_heuristic(
            "chat.message",
            &serde_json::json!({}),
            "hello world",
        );
        assert_eq!(plan.len(), 1);
        let text = plan[0]
            .params
            .get("ops")
            .and_then(|v| v.as_array())
            .and_then(|ops| ops.first())
            .and_then(|op| op.get("props"))
            .and_then(|p| p.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(text.contains("You: hello world"));
    }
}
