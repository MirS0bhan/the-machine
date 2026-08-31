//! Plan construction: heuristic fallback + synthesis helpers.

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub action: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

pub fn uses_heuristic_plan(intent: &str) -> bool {
    matches!(
        intent,
        // chat.message is handled separately: LLM reply → ui.patch (see main process_wake).
        "boot.greet" | "heartbeat" | "calculator" | "notification.triage"
    )
}

/// Fallback assistant line when cloud and localmodel are unavailable.
pub fn heuristic_chat_reply(user_text: &str) -> String {
    let trimmed = user_text.trim();
    if trimmed.is_empty() {
        "I'm here. Type a message and I'll reply locally when a model is available.".into()
    } else if trimmed.chars().count() < 80
        && (trimmed.ends_with('?')
            || trimmed.to_lowercase().starts_with("what")
            || trimmed.to_lowercase().starts_with("how")
            || trimmed.to_lowercase().starts_with("who"))
    {
        format!(
            "I heard: \"{trimmed}\". No cloud or local model is configured yet, so this is a local stub — add a key at /run/the-machine/secrets/cloud-api-key (0600) or enable localmodel."
        )
    } else {
        format!(
            "I received: \"{trimmed}\". Running without an LLM backend — cloud key or localmodel will unlock real replies."
        )
    }
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
        "chat.message" => chat_message_plan(text, &heuristic_chat_reply(text)),
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

/// Patch `#ui.chat_log` with the user line and assistant reply (same spine as boot.greet).
pub fn chat_message_plan(user_text: &str, assistant_reply: &str) -> Vec<PlanStep> {
    let reply = assistant_reply.trim();
    let user_line = if reply.is_empty() {
        format!("You: {user_text}")
    } else {
        format!("You: {user_text}\nAssistant: {reply}")
    };
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
        let plan = chat_message_plan("hello world", "Hi there.");
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].action, "ui.patch");
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
        assert!(text.contains("Assistant: Hi there."));
        assert!(!text.contains("LLM reply wiring is next"));
    }

    #[test]
    fn chat_message_not_forced_heuristic() {
        assert!(!uses_heuristic_plan("chat.message"));
        assert!(uses_heuristic_plan("boot.greet"));
    }

    #[test]
    fn heuristic_chat_reply_mentions_missing_backend() {
        let reply = heuristic_chat_reply("hello");
        assert!(reply.contains("hello"));
        assert!(
            reply.contains("LLM") || reply.contains("cloud") || reply.contains("localmodel"),
            "expected backend hint, got {reply}"
        );
    }
}
