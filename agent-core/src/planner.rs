//! Plan construction: heuristic fallback + desktop shell helpers.

use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub action: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// Max characters kept in the on-screen chat log (older turns dropped from the head).
pub const CHAT_LOG_MAX_CHARS: usize = 2400;

pub fn uses_heuristic_plan(intent: &str) -> bool {
    matches!(
        intent,
        // chat.message / generic use LLM reply + optional desktop actions (see main).
        "boot.greet" | "heartbeat" | "calculator" | "notification.triage" | "desktop.status" | "desktop.spawn"
    )
}

/// Fallback assistant line when cloud and localmodel are unavailable.
pub fn heuristic_chat_reply(user_text: &str) -> String {
    let trimmed = user_text.trim();
    if trimmed.is_empty() {
        "I'm here. Ask for system status, a workspace control, or a question — I'll reply locally when a model is available.".into()
    } else if looks_like_desktop_action(trimmed) {
        format!(
            "Understood: \"{trimmed}\". I'll update the desktop workspace with the matching controls or status."
        )
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

/// Append a turn to the prior chat log text, trimming from the head when oversized.
pub fn append_chat_log(prior: &str, user_text: &str, assistant_reply: &str) -> String {
    let reply = assistant_reply.trim();
    let turn = if reply.is_empty() {
        format!("You: {user_text}")
    } else {
        format!("You: {user_text}\nAssistant: {reply}")
    };
    let combined = if prior.trim().is_empty() {
        turn
    } else {
        format!("{}\n{}", prior.trim_end(), turn)
    };
    truncate_chat_log(&combined)
}

fn truncate_chat_log(log: &str) -> String {
    if log.len() <= CHAT_LOG_MAX_CHARS {
        return log.to_string();
    }
    let mut start = log.len().saturating_sub(CHAT_LOG_MAX_CHARS);
    while start < log.len() && !log.is_char_boundary(start) {
        start += 1;
    }
    let sliced = &log[start..];
    if let Some(nl) = sliced.find('\n') {
        format!("…{}", &sliced[nl + 1..])
    } else {
        format!("…{sliced}")
    }
}

pub fn looks_like_desktop_action(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("status")
        || t.contains("workspace")
        || t.contains("add a button")
        || t.contains("create a button")
        || t.contains("show a list")
        || t.contains("open a dialog")
        || t.contains("show dialog")
        || t.contains("list interfaces")
        || t.contains("network")
        || t.contains("what can you do")
        || t.contains("spawn")
}

/// Map free-text to a desktop-oriented intent when classifier is unavailable.
pub fn desktop_intent_from_text(text: &str) -> Option<&'static str> {
    let t = text.to_lowercase();
    if t.contains("add a button")
        || t.contains("create a button")
        || t.contains("show a list")
        || t.contains("open a dialog")
        || t.contains("show dialog")
        || t.contains("spawn")
        || t.contains("workspace")
    {
        Some("desktop.spawn")
    } else if t.contains("status")
        || t.contains("list interfaces")
        || t.contains("network")
        || t.contains("what can you do")
    {
        Some("desktop.status")
    } else {
        None
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
                        "anchor": "ui.workspace",
                        "node": {
                            "id": "ui.notification_card",
                            "type": "text",
                            "props": {
                                "role": "body",
                                "text": format!("Notification: {payload}")
                            },
                            "children": []
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
            activity_plan("Notification triaged into workspace"),
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
        "desktop.status" => desktop_status_plan(text),
        "desktop.spawn" => desktop_spawn_plan(text),
        "chat.message" => chat_message_plan(text, &heuristic_chat_reply(text), ""),
        _ => vec![
            PlanStep {
                action: "state.set".into(),
                params: serde_json::json!({ "path": "task.last_intent", "value": intent }),
            },
            activity_plan(&format!("Intent recorded: {intent}")),
        ],
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
                    "anchor": "ui.workspace",
                    "node": {
                        "id": widget_id,
                        "type": "button",
                        "props": { "label": text, "variant": "primary" },
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
        activity_plan(&format!("Registered {name} and placed control in workspace")),
    ]
}

fn boot_greet_plan() -> Vec<PlanStep> {
    let welcome = "Assistant: Welcome aboard. This is your agentic desktop — ask questions, request status, or ask me to place controls in the workspace.";
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
                        "props": { "text": welcome }
                    },
                    {
                        "op": "update",
                        "id": "ui.status_line",
                        "props": { "text": "The Machine · session ready" }
                    },
                    {
                        "op": "update",
                        "id": "ui.activity",
                        "props": { "text": "Boot greet complete" }
                    },
                    {
                        "op": "update",
                        "id": "ui.workspace_hint",
                        "props": { "text": "Workspace — agent-placed controls appear here" }
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
        PlanStep {
            action: "state.set".into(),
            params: serde_json::json!({
                "path": "task.chat_log",
                "value": welcome
            }),
        },
    ]
}

/// Patch `#ui.chat_log` by appending the user line and assistant reply; persist to state.
pub fn chat_message_plan(user_text: &str, assistant_reply: &str, prior_log: &str) -> Vec<PlanStep> {
    let log = append_chat_log(prior_log, user_text, assistant_reply);
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "update",
                    "id": "ui.chat_log",
                    "props": { "text": log }
                }]
            }),
        },
        PlanStep {
            action: "state.set".into(),
            params: serde_json::json!({
                "path": "task.chat_log",
                "value": log
            }),
        },
    ]
}

/// Merge a chat acknowledgment with follow-on MCP/desktop steps.
pub fn agentic_turn_plan(
    user_text: &str,
    assistant_reply: &str,
    prior_log: &str,
    follow_on: Vec<PlanStep>,
) -> Vec<PlanStep> {
    let mut plan = chat_message_plan(user_text, assistant_reply, prior_log);
    plan.extend(follow_on);
    plan
}

pub fn activity_plan(message: &str) -> PlanStep {
    PlanStep {
        action: "ui.patch".into(),
        params: serde_json::json!({
            "ops": [{
                "op": "update",
                "id": "ui.activity",
                "props": { "text": message }
            }]
        }),
    }
}

pub fn desktop_status_plan(text: &str) -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [
                    {
                        "op": "update",
                        "id": "ui.status_line",
                        "props": { "text": "The Machine · status" }
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "node": {
                            "id": "ui.status_panel",
                            "type": "list",
                            "props": {
                                "label": "Session actions",
                                "items": [
                                    "agent.status — model + wake counts",
                                    "ui.status — tree revision",
                                    "Ask in chat for network or power details"
                                ]
                            },
                            "children": [],
                            "bindings": [{
                                "type": "mcp",
                                "target": "agent.status"
                            }]
                        }
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "node": {
                            "id": "ui.status_refresh",
                            "type": "button",
                            "props": { "label": "Refresh agent status", "variant": "primary" },
                            "bindings": [{
                                "type": "mcp",
                                "target": "agent.status"
                            }]
                        }
                    }
                ]
            }),
        },
        PlanStep {
            action: "agent.status".into(),
            params: serde_json::json!({}),
        },
        activity_plan(&format!("Desktop status for: {}", text.trim())),
    ]
}

pub fn desktop_spawn_plan(text: &str) -> Vec<PlanStep> {
    let t = text.to_lowercase();
    if t.contains("dialog") {
        return vec![
            PlanStep {
                action: "ui.patch".into(),
                params: serde_json::json!({
                    "ops": [
                        {
                            "op": "insert",
                            "anchor": "ui.workspace",
                            "node": {
                                "id": "ui.agent_dialog",
                                "type": "dialog",
                                "props": {
                                    "label": "Agent dialog",
                                    "text": text,
                                    "dismissible": true
                                },
                                "children": []
                            }
                        },
                        {
                            "op": "insert",
                            "anchor": "ui.agent_dialog",
                            "node": {
                                "id": "ui.agent_dialog_ok",
                                "type": "button",
                                "props": { "label": "Dismiss", "variant": "primary" },
                                "bindings": [{
                                    "type": "mcp",
                                    "target": "ui.status"
                                }]
                            }
                        }
                    ]
                }),
            },
            activity_plan("Spawned dialog into workspace"),
        ];
    }
    if t.contains("list") {
        return vec![
            PlanStep {
                action: "ui.patch".into(),
                params: serde_json::json!({
                    "ops": [
                        {
                            "op": "insert",
                            "anchor": "ui.workspace",
                            "node": {
                                "id": "ui.agent_list",
                                "type": "list",
                                "props": {
                                    "label": "Agent list",
                                    "items": [
                                        "Refresh status",
                                        "Open calculator",
                                        "Clear workspace hint"
                                    ]
                                },
                                "children": []
                            }
                        },
                        {
                            "op": "insert",
                            "anchor": "ui.workspace",
                            "node": {
                                "id": "ui.agent_list_status",
                                "type": "button",
                                "props": { "label": "Call agent.status", "variant": "primary" },
                                "bindings": [{
                                    "type": "mcp",
                                    "target": "agent.status"
                                }]
                            }
                        }
                    ]
                }),
            },
            activity_plan("Spawned list + action button into workspace"),
        ];
    }
    // Default: primary button bound to a real MCP handler.
    let label = if text.trim().is_empty() {
        "Agent action".to_string()
    } else {
        text.chars().take(40).collect::<String>()
    };
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": {
                        "id": "ui.agent_button",
                        "type": "button",
                        "props": { "label": label, "variant": "primary" },
                        "bindings": [{
                            "type": "mcp",
                            "target": "agent.status"
                        }]
                    }
                }]
            }),
        },
        activity_plan("Spawned actionable button into workspace"),
    ]
}

/// Extra MCP steps inferred from conversational text (merged into chat turns).
pub fn desktop_actions_for_text(text: &str) -> Vec<PlanStep> {
    match desktop_intent_from_text(text) {
        Some("desktop.status") => desktop_status_plan(text),
        Some("desktop.spawn") => desktop_spawn_plan(text),
        _ => Vec::new(),
    }
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
        let patch = plan.iter().find(|s| s.action == "ui.patch").unwrap();
        let anchor = patch
            .params
            .get("ops")
            .and_then(|v| v.as_array())
            .and_then(|ops| ops.first())
            .and_then(|op| op.get("anchor"))
            .and_then(|v| v.as_str());
        assert_eq!(anchor, Some("ui.workspace"));
    }

    #[test]
    fn boot_greet_plan_updates_chat_and_chrome() {
        let plan = build_plan_heuristic("boot.greet", &serde_json::json!({}), "");
        assert!(plan.len() >= 2);
        assert_eq!(plan[0].action, "ui.patch");
        let ops = plan[0].params.get("ops").and_then(|v| v.as_array()).unwrap();
        assert!(ops.iter().any(|op| {
            op.get("id").and_then(|v| v.as_str()) == Some("ui.chat_log")
        }));
        assert!(ops.iter().any(|op| {
            op.get("id").and_then(|v| v.as_str()) == Some("ui.status_line")
        }));
        assert!(ops.iter().any(|op| {
            op.get("id").and_then(|v| v.as_str()) == Some("ui.activity")
        }));
    }

    #[test]
    fn chat_message_plan_appends_across_turns() {
        let prior = "You: hi\nAssistant: hello";
        let plan = chat_message_plan("second", "again", prior);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].action, "ui.patch");
        assert_eq!(plan[1].action, "state.set");
        let text = plan[0]
            .params
            .get("ops")
            .and_then(|v| v.as_array())
            .and_then(|ops| ops.first())
            .and_then(|op| op.get("props"))
            .and_then(|p| p.get("text"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        assert!(text.contains("You: hi"));
        assert!(text.contains("You: second"));
        assert!(text.contains("Assistant: again"));
        assert!(!text.contains("LLM reply wiring is next"));
    }

    #[test]
    fn append_chat_log_truncates_from_head() {
        let prior = "A".repeat(CHAT_LOG_MAX_CHARS);
        let out = append_chat_log(&prior, "new", "ok");
        assert!(out.len() <= CHAT_LOG_MAX_CHARS + 32);
        assert!(out.contains("You: new"));
    }

    #[test]
    fn chat_message_not_forced_heuristic() {
        assert!(!uses_heuristic_plan("chat.message"));
        assert!(uses_heuristic_plan("boot.greet"));
        assert!(uses_heuristic_plan("desktop.status"));
        assert!(uses_heuristic_plan("desktop.spawn"));
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

    #[test]
    fn desktop_spawn_places_mcp_bound_button() {
        let plan = desktop_spawn_plan("add a button for status");
        assert!(plan.iter().any(|s| s.action == "ui.patch"));
        let ops = plan[0].params.get("ops").and_then(|v| v.as_array()).unwrap();
        let node = ops[0].get("node").unwrap();
        assert_eq!(node.get("type").and_then(|v| v.as_str()), Some("button"));
        let target = node
            .get("bindings")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|b| b.get("target"))
            .and_then(|v| v.as_str());
        assert_eq!(target, Some("agent.status"));
    }

    #[test]
    fn desktop_status_calls_agent_status() {
        let plan = desktop_status_plan("show status");
        assert!(plan.iter().any(|s| s.action == "agent.status"));
        assert!(plan.iter().any(|s| s.action == "ui.patch"));
    }

    #[test]
    fn agentic_turn_merges_chat_and_actions() {
        let plan = agentic_turn_plan(
            "show status",
            "Pulling status.",
            "",
            desktop_status_plan("show status"),
        );
        assert!(plan.iter().any(|s| s.action == "ui.patch"));
        assert!(plan.iter().any(|s| s.action == "agent.status"));
        assert!(plan.iter().any(|s| s.action == "state.set"));
    }

    #[test]
    fn desktop_intent_detects_spawn_and_status() {
        assert_eq!(
            desktop_intent_from_text("please add a button"),
            Some("desktop.spawn")
        );
        assert_eq!(
            desktop_intent_from_text("what's the status"),
            Some("desktop.status")
        );
        assert_eq!(desktop_intent_from_text("hello there"), None);
    }
}
