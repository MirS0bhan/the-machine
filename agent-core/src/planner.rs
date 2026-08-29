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
