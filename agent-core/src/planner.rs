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
        "boot.greet"
            | "heartbeat"
            | "calculator"
            | "notification.triage"
            | "desktop.status"
            | "desktop.spawn"
            | "desktop.clear"
            | "desktop.system"
            | "chat.undo"
            | "chat.regenerate"
            | "chat.pin"
            | "chat.export"
            | "chat.suggestions"
            | "desktop.tour"
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
    desktop_intent_from_text(text).is_some() || chat_command_from_text(text).is_some()
}

/// Map free-text to a desktop-oriented intent when classifier is unavailable.
pub fn desktop_intent_from_text(text: &str) -> Option<&'static str> {
    let t = text.to_lowercase();
    if chat_command_from_text(text).is_some() {
        return chat_command_from_text(text);
    }
    if (t.contains("clear") || t.contains("reset") || t.contains("empty"))
        && (t.contains("workspace") || t.contains("controls"))
    {
        return Some("desktop.clear");
    }
    if t.contains("power")
        || t.contains("battery")
        || t.contains("volume")
        || t.contains("audio")
        || t.contains("wifi")
        || t.contains("connect wifi")
        || t.contains("display mode")
        || t.contains("brightness")
        || t.contains("set profile")
    {
        return Some("desktop.system");
    }
    if t.contains("add a button")
        || t.contains("create a button")
        || t.contains("show a list")
        || t.contains("open a dialog")
        || t.contains("show dialog")
        || t.contains("add a toggle")
        || t.contains("add a slider")
        || t.contains("show a chart")
        || t.contains("show a media")
        || t.contains("media panel")
        || t.contains("add an icon")
        || t.contains("lay out a grid")
        || t.contains("grid of actions")
        || t.contains("add another input")
        || t.contains("nest a stack")
        || t.contains("add a caption")
        || t.contains("spawn")
        || t.contains("workspace slider")
        || t.contains("workspace chart")
        || t.contains("workspace grid")
        || t.contains("workspace menu")
        || t.contains("workspace sidebar")
        || t.contains("show a menu")
        || t.contains("show a sidebar")
        || (t.contains("workspace") && !t.contains("clear") && !t.contains("status"))
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

/// Conversational chat-log commands (undo / pin / export / …).
pub fn chat_command_from_text(text: &str) -> Option<&'static str> {
    let t = text.to_lowercase();
    if t.contains("undo") || t.contains("edit last") {
        Some("chat.undo")
    } else if t.contains("regenerate") {
        Some("chat.regenerate")
    } else if t.contains("pin this") || t.contains("pin last") || t == "pin" {
        Some("chat.pin")
    } else if t.contains("export") {
        Some("chat.export")
    } else if t.contains("suggest") || t.contains("suggestion") {
        Some("chat.suggestions")
    } else if t.contains("take a tour") || t.contains("discover") || t.contains("what can i do") {
        Some("desktop.tour")
    } else if t.contains("list skills") || t.contains("@skill") || t.contains("skill mention") {
        Some("chat.suggestions")
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
        "boot.greet" => boot_greet_plan(
            payload
                .get("prior_log")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        ),
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
        "desktop.clear" => desktop_clear_workspace_plan(),
        "desktop.system" => desktop_system_plan(text),
        "chat.undo" => chat_undo_plan(
            payload
                .get("prior_log")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        ),
        "chat.regenerate" => chat_regenerate_plan(
            payload
                .get("prior_log")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            text,
        ),
        "chat.pin" => chat_pin_plan(
            payload
                .get("prior_log")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        ),
        "chat.export" => chat_export_plan(
            payload
                .get("prior_log")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        ),
        "chat.suggestions" => chat_suggestions_plan(),
        "desktop.tour" => desktop_tour_plan(),
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
        activity_plan(&format!(
            "Registered {name} and placed control in workspace"
        )),
    ]
}

fn boot_greet_plan(prior_log: &str) -> Vec<PlanStep> {
    let welcome = "Assistant: Welcome aboard. This is your agentic desktop — ask questions, request status, or ask me to place controls in the workspace.";
    let log = if prior_log.trim().is_empty() {
        welcome.to_string()
    } else {
        prior_log.to_string()
    };
    let items = chat_log_items(&log);
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
                        "props": {
                            "text": log,
                            "items": items,
                            "label": "Conversation",
                            "live": "polite"
                        }
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
                "value": log
            }),
        },
    ]
}

/// Patch `#ui.chat_log` by appending the user line and assistant reply; persist to state.
pub fn chat_message_plan(user_text: &str, assistant_reply: &str, prior_log: &str) -> Vec<PlanStep> {
    let log = append_chat_log(prior_log, user_text, assistant_reply);
    chat_log_patch_plan(&log)
}

/// Patch + persist an already-composed chat log (used by undo/restore).
pub fn chat_log_patch_plan(log: &str) -> Vec<PlanStep> {
    let items = chat_log_items(log);
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "update",
                    "id": "ui.chat_log",
                    "props": {
                        "text": log,
                        "items": items,
                        "label": "Conversation",
                        "live": "polite"
                    }
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

pub fn chat_log_items(log: &str) -> Vec<String> {
    log.lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "…")
        .map(|s| s.to_string())
        .collect()
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
    let t = text.to_lowercase();
    let network_focused = t.contains("network") || t.contains("interfaces");
    let mut items = vec![
        "agent.status — model + wake counts".to_string(),
        "ui.status — tree revision".to_string(),
    ];
    if network_focused {
        items.push("net.list_interfaces — fetch live interface list".to_string());
    } else {
        items.push("Ask in chat for network or power details".to_string());
    }
    let items_json: Vec<serde_json::Value> =
        items.into_iter().map(serde_json::Value::from).collect();
    let mut steps = vec![
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
                                "label": if network_focused { "Network & session" } else { "Session actions" },
                                "items": items_json
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
    ];
    if network_focused {
        steps.insert(
            2,
            PlanStep {
                action: "net.list_interfaces".into(),
                params: serde_json::json!({}),
            },
        );
    }
    steps
}

pub fn desktop_clear_workspace_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.workspace.clear".into(),
            params: serde_json::json!({ "preserve_hint": true }),
        },
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "update",
                    "id": "ui.workspace_hint",
                    "props": { "text": "Workspace cleared — ask to place new controls" }
                }]
            }),
        },
        activity_plan("Cleared workspace controls"),
    ]
}

pub fn desktop_spawn_plan(text: &str) -> Vec<PlanStep> {
    let t = text.to_lowercase();
    if t.contains("dialog") {
        return spawn_dialog_plan(text);
    }
    if t.contains("list") {
        return spawn_list_plan();
    }
    if t.contains("toggle") {
        return spawn_primitive_plan(
            "ui.agent_toggle",
            "toggle",
            serde_json::json!({
                "label": "Agent toggle",
                "checked": false
            }),
            "Spawned toggle into workspace",
        );
    }
    if t.contains("slider") {
        return spawn_primitive_plan(
            "ui.agent_slider",
            "slider",
            serde_json::json!({
                "label": "Agent slider",
                "min": 0,
                "max": 100,
                "value": 50
            }),
            "Spawned slider into workspace",
        );
    }
    if t.contains("media") || t.contains("video") {
        return spawn_primitive_plan(
            "ui.agent_media",
            "media",
            serde_json::json!({
                "label": "Agent media",
                "src": ""
            }),
            "Spawned media panel into workspace",
        );
    }
    if t.contains("chart") {
        return spawn_primitive_plan(
            "ui.agent_chart",
            "chart",
            serde_json::json!({
                "label": "Agent chart",
                "data": [12, 28, 19, 34, 22]
            }),
            "Spawned chart into workspace",
        );
    }
    if t.contains("icon") {
        return spawn_primitive_plan(
            "ui.agent_icon",
            "icon",
            serde_json::json!({
                "label": "Agent icon",
                "variant": "lg",
                "glyph": "star"
            }),
            "Spawned icon into workspace",
        );
    }
    if t.contains("grid") {
        return spawn_grid_plan();
    }
    if t.contains("field") || t.contains("input") {
        return spawn_primitive_plan(
            "ui.agent_field",
            "field",
            serde_json::json!({
                "placeholder": "Workspace field",
                "text": ""
            }),
            "Spawned field into workspace",
        );
    }
    if t.contains("stack") || t.contains("nest") {
        return spawn_stack_plan();
    }
    if t.contains("menu") {
        return spawn_menu_plan();
    }
    if t.contains("sidebar") {
        return spawn_sidebar_plan();
    }
    if t.contains("caption") || (t.contains("text") && !t.contains("context")) {
        let caption = text.chars().take(80).collect::<String>();
        return spawn_primitive_plan(
            "ui.agent_caption",
            "text",
            serde_json::json!({
                "role": "body",
                "text": if caption.trim().is_empty() {
                    "Agent caption".to_string()
                } else {
                    caption
                }
            }),
            "Spawned caption text into workspace",
        );
    }
    spawn_button_plan(text)
}

fn spawn_primitive_plan(
    id: &str,
    kind: &str,
    props: serde_json::Value,
    activity: &str,
) -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": {
                        "id": id,
                        "type": kind,
                        "props": props,
                        "children": []
                    }
                }]
            }),
        },
        activity_plan(activity),
    ]
}

fn spawn_dialog_plan(text: &str) -> Vec<PlanStep> {
    vec![
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
    ]
}

fn spawn_list_plan() -> Vec<PlanStep> {
    vec![
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
    ]
}

fn spawn_grid_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "node": {
                            "id": "ui.agent_grid",
                            "type": "grid",
                            "props": { "cols": 2, "gap": "md" },
                            "children": ["ui.agent_grid_a", "ui.agent_grid_b"]
                        }
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.agent_grid",
                        "node": {
                            "id": "ui.agent_grid_a",
                            "type": "button",
                            "props": { "label": "Status", "variant": "primary" },
                            "bindings": [{ "type": "mcp", "target": "agent.status" }]
                        }
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.agent_grid",
                        "node": {
                            "id": "ui.agent_grid_b",
                            "type": "button",
                            "props": { "label": "Clear", "variant": "secondary" },
                            "bindings": [{ "type": "mcp", "target": "ui.workspace.clear" }]
                        }
                    }
                ]
            }),
        },
        activity_plan("Spawned 2-column action grid into workspace"),
    ]
}

fn spawn_stack_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "node": {
                            "id": "ui.agent_stack",
                            "type": "stack",
                            "props": { "dir": "v", "gap": "sm" },
                            "children": ["ui.agent_stack_label", "ui.agent_stack_btn"]
                        }
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.agent_stack",
                        "node": {
                            "id": "ui.agent_stack_label",
                            "type": "text",
                            "props": { "role": "caption", "text": "Nested stack controls" },
                            "children": []
                        }
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.agent_stack",
                        "node": {
                            "id": "ui.agent_stack_btn",
                            "type": "button",
                            "props": { "label": "Stack action", "variant": "primary" },
                            "bindings": [{ "type": "mcp", "target": "agent.status" }]
                        }
                    }
                ]
            }),
        },
        activity_plan("Spawned nested stack into workspace"),
    ]
}

fn spawn_button_plan(text: &str) -> Vec<PlanStep> {
    let label = if text.trim().is_empty() {
        "Agent action".to_string()
    } else {
        text.chars().take(40).collect::<String>()
    };
    let target = mcp_target_from_text(text);
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
                            "target": target
                        }]
                    }
                }]
            }),
        },
        activity_plan("Spawned actionable button into workspace"),
    ]
}

/// Parse `bind to <method>` / `call <method>` from chat; default agent.status.
pub fn mcp_target_from_text(text: &str) -> String {
    let lower = text.to_lowercase();
    for needle in ["bind to ", "bind ", "call "] {
        if let Some(idx) = lower.find(needle) {
            let rest = text[idx + needle.len()..].trim();
            let token = rest
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-'))
                .next()
                .unwrap_or("")
                .trim();
            if token.contains('.') && token.len() >= 3 {
                return token.to_string();
            }
        }
    }
    "agent.status".into()
}

fn spawn_menu_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": {
                        "id": "ui.agent_menu",
                        "type": "stack",
                        "props": { "dir": "v", "gap": "sm", "role": "menu" },
                        "children": ["ui.agent_menu_status", "ui.agent_menu_clear"]
                    }
                },
                {
                    "op": "insert",
                    "anchor": "ui.agent_menu",
                    "node": {
                        "id": "ui.agent_menu_status",
                        "type": "button",
                        "props": { "label": "Status", "variant": "primary" },
                        "bindings": [{ "type": "mcp", "target": "agent.status" }]
                    }
                },
                {
                    "op": "insert",
                    "anchor": "ui.agent_menu",
                    "node": {
                        "id": "ui.agent_menu_clear",
                        "type": "button",
                        "props": { "label": "Clear workspace", "variant": "secondary" },
                        "bindings": [{ "type": "mcp", "target": "ui.workspace.clear" }]
                    }
                }]
            }),
        },
        activity_plan("Spawned workspace menu (AUIL stack, not X11)"),
    ]
}

fn spawn_sidebar_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": {
                        "id": "ui.agent_sidebar",
                        "type": "stack",
                        "props": { "dir": "v", "gap": "md", "role": "navigation" },
                        "children": ["ui.agent_sidebar_title", "ui.agent_sidebar_btn"]
                    }
                },
                {
                    "op": "insert",
                    "anchor": "ui.agent_sidebar",
                    "node": {
                        "id": "ui.agent_sidebar_title",
                        "type": "text",
                        "props": { "role": "caption", "text": "Sidebar" },
                        "children": []
                    }
                },
                {
                    "op": "insert",
                    "anchor": "ui.agent_sidebar",
                    "node": {
                        "id": "ui.agent_sidebar_btn",
                        "type": "button",
                        "props": { "label": "Refresh", "variant": "primary" },
                        "bindings": [{ "type": "mcp", "target": "ui.status" }]
                    }
                }]
            }),
        },
        activity_plan("Spawned workspace sidebar stack"),
    ]
}

pub fn desktop_system_plan(text: &str) -> Vec<PlanStep> {
    let t = text.to_lowercase();
    if t.contains("volume") || t.contains("audio") {
        return volume_plan();
    }
    if t.contains("wifi") {
        return wifi_plan();
    }
    if t.contains("display") || t.contains("brightness") {
        return display_plan();
    }
    power_plan()
}

fn power_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "power.get_profile".into(),
            params: serde_json::json!({}),
        },
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "node": {
                            "id": "ui.power_panel",
                            "type": "list",
                            "props": {
                                "label": "Power",
                                "items": ["Fetching power profile…"]
                            },
                            "children": []
                        }
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "node": {
                            "id": "ui.battery_widget",
                            "type": "text",
                            "props": {
                                "role": "caption",
                                "text": "Battery: querying host sysfs via power.get_profile"
                            },
                            "children": []
                        }
                    }
                ]
            }),
        },
        activity_plan("Power profile requested"),
    ]
}

fn volume_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "audio.list_devices".into(),
            params: serde_json::json!({}),
        },
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "node": {
                            "id": "ui.volume_slider",
                            "type": "slider",
                            "props": {
                                "label": "Volume",
                                "min": 0,
                                "max": 100,
                                "value": 50
                            },
                            "bindings": [{ "type": "mcp", "target": "audio.set_default" }]
                        }
                    }
                ]
            }),
        },
        activity_plan("Volume slider bound to audio.set_default"),
    ]
}

fn wifi_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "net.get_wifi_status".into(),
            params: serde_json::json!({}),
        },
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": {
                        "id": "ui.wifi_panel",
                        "type": "list",
                        "props": { "label": "Wi-Fi", "items": ["Fetching wifi status…"] },
                        "children": []
                    }
                }]
            }),
        },
        activity_plan("Wi-Fi status requested"),
    ]
}

fn display_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "display.get_modes".into(),
            params: serde_json::json!({}),
        },
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": {
                        "id": "ui.display_panel",
                        "type": "list",
                        "props": { "label": "Display modes", "items": ["Fetching modes…"] },
                        "children": []
                    }
                }]
            }),
        },
        activity_plan("Display modes requested"),
    ]
}

pub fn chat_undo_plan(prior: &str) -> Vec<PlanStep> {
    let next = strip_last_turn(prior);
    let mut steps = chat_log_patch_plan(&next);
    steps.push(activity_plan("Undid last chat turn"));
    steps
}

pub fn chat_regenerate_plan(prior: &str, text: &str) -> Vec<PlanStep> {
    let (without_last, last_user) = split_last_user_turn(prior);
    let prompt = if last_user.is_empty() {
        text.to_string()
    } else {
        last_user
    };
    let reply = heuristic_chat_reply(&prompt);
    let mut steps = chat_message_plan(&prompt, &format!("(regenerated) {reply}"), &without_last);
    steps.push(activity_plan("Regenerated last assistant turn"));
    steps
}

pub fn chat_pin_plan(prior: &str) -> Vec<PlanStep> {
    let last = last_assistant_line(prior).unwrap_or_else(|| "Nothing to pin yet".into());
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": {
                        "id": "ui.pinned_turn",
                        "type": "text",
                        "props": { "role": "body", "text": format!("Pinned: {last}") },
                        "children": []
                    }
                }]
            }),
        },
        activity_plan("Pinned last assistant turn into workspace"),
    ]
}

pub fn chat_export_plan(prior: &str) -> Vec<PlanStep> {
    let items = chat_log_items(prior);
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": {
                        "id": "ui.chat_export",
                        "type": "list",
                        "props": { "label": "Exported conversation", "items": items },
                        "children": []
                    }
                }]
            }),
        },
        PlanStep {
            action: "state.set".into(),
            params: serde_json::json!({ "path": "task.chat_export", "value": prior }),
        },
        activity_plan("Exported conversation into workspace list"),
    ]
}

pub fn chat_suggestions_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "node": {
                        "id": "ui.suggestion_tray",
                        "type": "list",
                        "props": {
                            "label": "Suggestions",
                            "items": [
                                "show status",
                                "add a toggle",
                                "clear workspace",
                                "list interfaces",
                                "take a tour"
                            ]
                        },
                        "children": [],
                        "bindings": [{ "type": "mcp", "target": "agent.chat.send" }]
                    }
                }]
            }),
        },
        activity_plan("Opened suggestion tray"),
    ]
}

pub fn desktop_tour_plan() -> Vec<PlanStep> {
    let mut steps = chat_suggestions_plan();
    steps.insert(
        0,
        PlanStep {
            action: "ui.patch".into(),
            params: serde_json::json!({
                "ops": [{
                    "op": "update",
                    "id": "ui.workspace_hint",
                    "props": {
                        "text": "Tour: chat here, place controls in the workspace, Tab to focus, Enter to send."
                    }
                }]
            }),
        },
    );
    steps.push(activity_plan("Session tour shown"));
    steps
}

pub fn strip_last_turn(prior: &str) -> String {
    let lines: Vec<&str> = prior.lines().collect();
    if lines.is_empty() {
        return String::new();
    }
    let mut end = lines.len();
    // Drop trailing Assistant line(s) then the matching You line.
    if lines[end - 1].starts_with("Assistant:") {
        end -= 1;
    }
    if end > 0 && lines[end - 1].starts_with("You:") {
        end -= 1;
    }
    lines[..end].join("\n")
}

pub fn split_last_user_turn(prior: &str) -> (String, String) {
    let stripped = strip_last_turn(prior);
    let last_user = prior
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("You: ").map(|s| s.to_string()))
        .unwrap_or_default();
    (stripped, last_user)
}

fn last_assistant_line(prior: &str) -> Option<String> {
    prior
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("Assistant: ").map(|s| s.to_string()))
}

/// Patch power panel after `power.get_profile`.
pub fn power_status_patch_plan(result: &serde_json::Value) -> Vec<PlanStep> {
    let profile = result
        .get("profile")
        .or_else(|| result.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    vec![PlanStep {
        action: "ui.patch".into(),
        params: serde_json::json!({
            "ops": [
                {
                    "op": "update",
                    "id": "ui.power_panel",
                    "props": { "items": [format!("profile: {profile}"), "powersave / balanced / performance"] }
                },
                {
                    "op": "update",
                    "id": "ui.battery_widget",
                    "props": { "text": format!("Power profile: {profile}") }
                }
            ]
        }),
    }]
}

/// Patch wifi panel after `net.get_wifi_status`.
pub fn wifi_status_patch_plan(result: &serde_json::Value) -> Vec<PlanStep> {
    let ssid = result
        .get("ssid")
        .and_then(|v| v.as_str())
        .unwrap_or("not connected");
    let state = result
        .get("state")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    vec![PlanStep {
        action: "ui.patch".into(),
        params: serde_json::json!({
            "ops": [{
                "op": "update",
                "id": "ui.wifi_panel",
                "props": { "items": [format!("ssid: {ssid}"), format!("state: {state}")] }
            }]
        }),
    }]
}

/// Patch display panel after `display.get_modes`.
pub fn display_modes_patch_plan(result: &serde_json::Value) -> Vec<PlanStep> {
    let mut items = Vec::new();
    if let Some(arr) = result.get("modes").and_then(|v| v.as_array()) {
        for m in arr.iter().take(12) {
            if let Some(s) = m.as_str() {
                items.push(s.to_string());
            } else if let Some(w) = m.get("width") {
                let h = m.get("height").and_then(|v| v.as_u64()).unwrap_or(0);
                items.push(format!("{}x{}", w, h));
            }
        }
    }
    if items.is_empty() {
        items.push("No DRM modes (host-dependent)".into());
    }
    vec![PlanStep {
        action: "ui.patch".into(),
        params: serde_json::json!({
            "ops": [{ "op": "update", "id": "ui.display_panel", "props": { "items": items } }]
        }),
    }]
}

/// Format `net.list_interfaces` MCP result into workspace list items.
pub fn network_interface_items(result: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(arr) = result
        .get("interfaces")
        .or_else(|| result.get("value"))
        .and_then(|v| v.as_array())
    {
        for iface in arr {
            if let Some(name) = iface.get("name").and_then(|v| v.as_str()) {
                let state = iface
                    .get("state")
                    .or_else(|| iface.get("operstate"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let addr = iface
                    .get("addresses")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if addr.is_empty() {
                    out.push(format!("{name}: {state}"));
                } else {
                    out.push(format!("{name}: {state} ({addr})"));
                }
            }
        }
    }
    if out.is_empty() {
        out.push("No interfaces reported (net.list_interfaces empty)".into());
    }
    out
}

/// Patch workspace with live network interface rows after net.list_interfaces.
pub fn network_status_patch_plan(items: &[String]) -> Vec<PlanStep> {
    let items_json: Vec<serde_json::Value> =
        items.iter().cloned().map(serde_json::Value::from).collect();
    vec![PlanStep {
        action: "ui.patch".into(),
        params: serde_json::json!({
            "ops": [
                {
                    "op": "replace",
                    "id": "ui.status_panel",
                    "node": {
                        "id": "ui.status_panel",
                        "type": "list",
                        "props": {
                            "label": "Network interfaces",
                            "items": items_json
                        },
                        "children": [],
                        "bindings": [{ "type": "mcp", "target": "net.list_interfaces" }]
                    }
                },
                {
                    "op": "update",
                    "id": "ui.activity",
                    "props": { "text": format!("Network: {} interface(s)", items.len()) }
                }
            ]
        }),
    }]
}

/// Extra MCP steps inferred from conversational text (merged into chat turns).
pub fn desktop_actions_for_text(text: &str) -> Vec<PlanStep> {
    match desktop_intent_from_text(text) {
        Some("desktop.status") => desktop_status_plan(text),
        Some("desktop.spawn") => desktop_spawn_plan(text),
        Some("desktop.clear") => desktop_clear_workspace_plan(),
        Some("desktop.system") => desktop_system_plan(text),
        Some("chat.undo") => Vec::new(), // handled as exclusive chat command
        Some("chat.regenerate") => Vec::new(),
        Some("chat.pin") => chat_pin_plan(""),
        Some("chat.export") => Vec::new(),
        Some("chat.suggestions") => chat_suggestions_plan(),
        Some("desktop.tour") => desktop_tour_plan(),
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
        let ops = plan[0]
            .params
            .get("ops")
            .and_then(|v| v.as_array())
            .unwrap();
        assert!(ops
            .iter()
            .any(|op| { op.get("id").and_then(|v| v.as_str()) == Some("ui.chat_log") }));
        assert!(ops
            .iter()
            .any(|op| { op.get("id").and_then(|v| v.as_str()) == Some("ui.status_line") }));
        assert!(ops
            .iter()
            .any(|op| { op.get("id").and_then(|v| v.as_str()) == Some("ui.activity") }));
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
    fn desktop_spawn_toggle_from_chat_text() {
        let plan = desktop_spawn_plan("add a toggle");
        let ops = plan[0]
            .params
            .get("ops")
            .and_then(|v| v.as_array())
            .unwrap();
        let node = ops[0].get("node").unwrap();
        assert_eq!(node.get("type").and_then(|v| v.as_str()), Some("toggle"));
    }

    #[test]
    fn desktop_spawn_slider_from_chat_text() {
        let plan = desktop_spawn_plan("add a slider");
        let ops = plan[0]
            .params
            .get("ops")
            .and_then(|v| v.as_array())
            .unwrap();
        let node = ops[0].get("node").unwrap();
        assert_eq!(node.get("type").and_then(|v| v.as_str()), Some("slider"));
    }

    #[test]
    fn desktop_spawn_grid_has_two_buttons() {
        let plan = desktop_spawn_plan("lay out a grid of actions");
        let ops = plan[0]
            .params
            .get("ops")
            .and_then(|v| v.as_array())
            .unwrap();
        assert!(ops.iter().any(|op| {
            op.get("node")
                .and_then(|n| n.get("type"))
                .and_then(|v| v.as_str())
                == Some("grid")
        }));
    }

    #[test]
    fn desktop_clear_workspace_calls_ui_workspace_clear() {
        let plan = desktop_clear_workspace_plan();
        assert!(plan.iter().any(|s| s.action == "ui.workspace.clear"));
    }

    #[test]
    fn desktop_intent_detects_clear_workspace() {
        assert_eq!(
            desktop_intent_from_text("please clear workspace"),
            Some("desktop.clear")
        );
    }

    #[test]
    fn network_interface_items_formats_names() {
        let raw = serde_json::json!({
            "interfaces": [
                { "name": "lo", "state": "up", "addresses": ["127.0.0.1"] },
                { "name": "eth0", "operstate": "down" }
            ]
        });
        let items = network_interface_items(&raw);
        assert!(items[0].contains("lo"));
        assert!(items[1].contains("eth0"));
    }

    #[test]
    fn desktop_status_network_includes_net_list() {
        let plan = desktop_status_plan("what is my network status");
        assert!(plan.iter().any(|s| s.action == "net.list_interfaces"));
    }

    #[test]
    fn desktop_spawn_places_mcp_bound_button() {
        let plan = desktop_spawn_plan("add a button for status");
        assert!(plan.iter().any(|s| s.action == "ui.patch"));
        let ops = plan[0]
            .params
            .get("ops")
            .and_then(|v| v.as_array())
            .unwrap();
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

    #[test]
    fn desktop_spawn_all_twelve_primitives() {
        for (msg, kind) in [
            ("add a toggle", "toggle"),
            ("add a slider", "slider"),
            ("show a media panel", "media"),
            ("show a chart", "chart"),
            ("add an icon", "icon"),
            ("lay out a grid", "grid"),
            ("add another input field", "field"),
            ("nest a stack", "stack"),
            ("add a caption text", "text"),
            ("open a dialog", "dialog"),
            ("show a list", "list"),
            ("add a button", "button"),
        ] {
            let plan = desktop_spawn_plan(msg);
            let ops = plan[0]
                .params
                .get("ops")
                .and_then(|v| v.as_array())
                .unwrap();
            assert!(
                ops.iter().any(|op| {
                    op.get("node")
                        .and_then(|n| n.get("type"))
                        .and_then(|v| v.as_str())
                        == Some(kind)
                }),
                "expected {kind} from {msg}"
            );
        }
    }

    #[test]
    fn bind_to_custom_mcp_target() {
        assert_eq!(
            mcp_target_from_text("add a button bind to net.list_interfaces"),
            "net.list_interfaces"
        );
        let plan = desktop_spawn_plan("create a button bind to power.get_profile");
        let ops = plan[0]
            .params
            .get("ops")
            .and_then(|v| v.as_array())
            .unwrap();
        let target = ops[0]
            .get("node")
            .and_then(|n| n.get("bindings"))
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|b| b.get("target"))
            .and_then(|v| v.as_str());
        assert_eq!(target, Some("power.get_profile"));
    }

    #[test]
    fn system_plan_calls_power_and_audio() {
        let power = desktop_system_plan("show battery");
        assert!(power.iter().any(|s| s.action == "power.get_profile"));
        let vol = desktop_system_plan("set volume");
        assert!(vol.iter().any(|s| s.action == "audio.list_devices"));
        assert!(vol.iter().any(|s| {
            s.params
                .get("ops")
                .and_then(|v| v.as_array())
                .and_then(|ops| ops.first())
                .and_then(|op| op.get("node"))
                .and_then(|n| n.get("type"))
                .and_then(|v| v.as_str())
                == Some("slider")
        }));
    }

    #[test]
    fn chat_undo_strips_last_turn() {
        let prior = "You: hi\nAssistant: hello\nYou: later\nAssistant: ok";
        let plan = chat_undo_plan(prior);
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
        assert!(!text.contains("You: later"));
    }

    #[test]
    fn chat_log_plan_sets_live_items() {
        let plan = chat_message_plan("hi", "hello", "");
        let props = plan[0]
            .params
            .get("ops")
            .and_then(|v| v.as_array())
            .and_then(|ops| ops.first())
            .and_then(|op| op.get("props"))
            .unwrap();
        assert_eq!(props.get("live").and_then(|v| v.as_str()), Some("polite"));
        assert!(props.get("items").and_then(|v| v.as_array()).is_some());
    }

    #[test]
    fn chat_commands_detected() {
        assert_eq!(chat_command_from_text("please undo"), Some("chat.undo"));
        assert_eq!(
            chat_command_from_text("export the chat"),
            Some("chat.export")
        );
        assert_eq!(chat_command_from_text("take a tour"), Some("desktop.tour"));
        assert_eq!(
            desktop_intent_from_text("clear the workspace"),
            Some("desktop.clear")
        );
        assert_eq!(
            desktop_intent_from_text("show battery"),
            Some("desktop.system")
        );
        assert_eq!(
            desktop_intent_from_text("show a menu"),
            Some("desktop.spawn")
        );
    }

    #[test]
    fn heuristic_plan_includes_new_intents() {
        assert!(uses_heuristic_plan("desktop.clear"));
        assert!(uses_heuristic_plan("desktop.system"));
        assert!(uses_heuristic_plan("chat.undo"));
        assert!(uses_heuristic_plan("desktop.tour"));
    }
}
