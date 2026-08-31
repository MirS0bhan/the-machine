//! Agent-driven desktop composition: spawn every AUIL primitive, manage the
//! workspace lifecycle, and build multi-step MCP plans from conversational text.
//!
//! The twelve primitives (`docs/design-system/03-widgets-and-types`) are the only
//! node kinds emitted here. Product surfaces the design system names separately
//! (`menu`, `sidebar`, `panel`, …) are **composed** from those primitives rather
//! than introducing a thirteenth kind.

use serde_json::{json, Value};

use crate::planner::{activity_plan, PlanStep};

/// The twelve AUIL primitives painted by the boot path.
pub const PRIMITIVES: [&str; 12] = [
    "stack", "grid", "text", "field", "button", "toggle", "slider", "list", "dialog", "icon",
    "media", "chart",
];

/// Product surface names the agent understands, mapped onto a primitive.
///
/// `alias` is the word the user said; `primitive` is what actually gets painted.
const SURFACE_ALIASES: [(&str, &str); 18] = [
    ("menu", "list"),
    ("context menu", "list"),
    ("sidebar", "stack"),
    ("panel", "stack"),
    ("card", "stack"),
    ("toolbar", "stack"),
    ("tray", "list"),
    ("checkbox", "toggle"),
    ("switch", "toggle"),
    ("radio", "toggle"),
    ("progress", "slider"),
    ("gauge", "slider"),
    ("graph", "chart"),
    ("plot", "chart"),
    ("video", "media"),
    ("image", "media"),
    ("player", "media"),
    ("label", "text"),
];

/// MCP methods the agent is allowed to bind freshly spawned controls to without
/// synthesising a lambda first. Everything else goes through `lambda.register`.
const KNOWN_BINDABLE: [&str; 12] = [
    "agent.status",
    "agent.chat.send",
    "agent.tour.next",
    "ui.status",
    "ui.workspace.clear",
    "ui.a11y.tree",
    "system-daemon.stats",
    "net.list_interfaces",
    "display.get_modes",
    "audio.list_devices",
    "power.get_profile",
    "clipboard.get",
];

/// Resolve the primitive a spawn request is asking for, plus the surface word used.
pub fn spawn_target(text: &str) -> (&'static str, String) {
    let t = text.to_lowercase();
    // Longest alias first so "context menu" beats "menu".
    let mut best: Option<(&'static str, &'static str)> = None;
    for (alias, primitive) in SURFACE_ALIASES {
        if t.contains(alias)
            && best
                .map(|(prev, _)| alias.len() > prev.len())
                .unwrap_or(true)
        {
            best = Some((alias, primitive));
        }
    }
    for primitive in PRIMITIVES {
        if t.contains(primitive)
            && best
                .map(|(prev, _)| primitive.len() > prev.len())
                .unwrap_or(true)
        {
            best = Some((primitive, primitive));
        }
    }
    match best {
        Some((alias, primitive)) => (primitive, alias.to_string()),
        None => ("button", "button".to_string()),
    }
}

/// Stable-but-unique node id for an agent-spawned control.
///
/// `seq` comes from `task.spawn_seq` so repeated spawns of the same kind never
/// collide; a caller that wants to *replace* an existing control passes its id
/// explicitly instead.
pub fn spawn_id(primitive: &str, seq: u64) -> String {
    format!("ui.agent_{primitive}_{seq}")
}

fn short_label(text: &str, fallback: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return fallback.to_string();
    }
    let cleaned: String = trimmed
        .chars()
        .take(48)
        .collect::<String>()
        .trim()
        .to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

/// Build the node JSON for `primitive`, including children for composite surfaces.
///
/// Returns `(node, extra_ops)` — `extra_ops` are follow-up `ui.patch` ops that
/// must be applied after the parent node exists (children of a dialog / stack).
pub fn spawn_node(primitive: &str, surface: &str, text: &str, id: &str) -> (Value, Vec<Value>) {
    let label = short_label(text, surface);
    let mut extra = Vec::new();
    let node = match primitive {
        "button" => json!({
            "id": id,
            "type": "button",
            "props": { "label": label, "variant": "primary", "surface": surface },
            "bindings": [{ "type": "mcp", "target": "agent.status", "event": "press" }],
            "children": [],
        }),
        "toggle" => json!({
            "id": id,
            "type": "toggle",
            "props": { "label": label, "checked": false, "surface": surface },
            "bindings": [{ "type": "mcp", "target": "agent.status", "event": "change" }],
            "children": [],
        }),
        "slider" => json!({
            "id": id,
            "type": "slider",
            "props": {
                "label": label,
                "min": 0.0,
                "max": 100.0,
                "value": 50.0,
                "surface": surface,
            },
            "bindings": [{ "type": "mcp", "target": "agent.status", "event": "change" }],
            "children": [],
        }),
        "field" => json!({
            "id": id,
            "type": "field",
            "props": {
                "label": label,
                "placeholder": label,
                "text": "",
                "caret": 0,
                "input-mode": "hybrid",
                "surface": surface,
            },
            "bindings": [{ "type": "mcp", "target": "agent.chat.send", "event": "submit" }],
            "children": [],
        }),
        "list" => json!({
            "id": id,
            "type": "list",
            "props": {
                "label": label,
                "items": list_items_for(surface, text),
                "selected": 0,
                "scroll_y": 0,
                "surface": surface,
            },
            "bindings": [{ "type": "mcp", "target": "agent.status", "event": "activate" }],
            "children": [],
        }),
        "dialog" => {
            extra.push(json!({
                "op": "insert",
                "anchor": id,
                "position": "child",
                "node": {
                    "id": format!("{id}_ok"),
                    "type": "button",
                    "props": { "label": "Dismiss", "variant": "primary" },
                    "bindings": [{ "type": "mcp", "target": "ui.status", "event": "press" }],
                    "children": [],
                },
            }));
            json!({
                "id": id,
                "type": "dialog",
                "props": {
                    "label": label,
                    "text": short_label(text, "Agent dialog"),
                    "dismissible": true,
                    "modal": true,
                    "surface": surface,
                },
                "children": [],
            })
        }
        "icon" => json!({
            "id": id,
            "type": "icon",
            "props": { "name": icon_name_for(text), "size": "md", "label": label, "surface": surface },
            "children": [],
        }),
        "media" => json!({
            "id": id,
            "type": "media",
            "props": {
                "label": label,
                "src": media_src_for(text),
                "poster": "",
                "surface": surface,
            },
            "children": [],
        }),
        "chart" => json!({
            "id": id,
            "type": "chart",
            "props": {
                "label": label,
                "data": [3, 7, 4, 9, 6, 8],
                "axis": "bars",
                "surface": surface,
            },
            "children": [],
        }),
        "text" => json!({
            "id": id,
            "type": "text",
            "props": { "role": "body", "text": short_label(text, "Agent note"), "surface": surface },
            "children": [],
        }),
        "grid" => {
            for n in 0..4 {
                extra.push(json!({
                    "op": "insert",
                    "anchor": id,
                    "position": "child",
                    "node": {
                        "id": format!("{id}_cell{n}"),
                        "type": "button",
                        "props": { "label": format!("Cell {}", n + 1), "variant": "secondary" },
                        "bindings": [{ "type": "mcp", "target": "agent.status", "event": "press" }],
                        "children": [],
                    },
                }));
            }
            json!({
                "id": id,
                "type": "grid",
                "props": { "cols": 2, "gap": "md", "label": label, "surface": surface },
                "children": [],
            })
        }
        // "stack" and anything unexpected compose a labelled container.
        _ => {
            extra.push(json!({
                "op": "insert",
                "anchor": id,
                "position": "child",
                "node": {
                    "id": format!("{id}_title"),
                    "type": "text",
                    "props": { "role": "caption", "text": label },
                    "children": [],
                },
            }));
            extra.push(json!({
                "op": "insert",
                "anchor": id,
                "position": "child",
                "node": {
                    "id": format!("{id}_action"),
                    "type": "button",
                    "props": { "label": "Agent status", "variant": "secondary" },
                    "bindings": [{ "type": "mcp", "target": "agent.status", "event": "press" }],
                    "children": [],
                },
            }));
            json!({
                "id": id,
                "type": "stack",
                "props": {
                    "dir": if surface == "toolbar" { "h" } else { "v" },
                    "gap": "md",
                    "label": label,
                    "surface": surface,
                },
                "children": [],
            })
        }
    };
    (node, extra)
}

fn list_items_for(surface: &str, text: &str) -> Vec<String> {
    match surface {
        "menu" | "context menu" => vec![
            "Refresh agent status".into(),
            "Clear workspace".into(),
            "Show accessibility tree".into(),
            "Copy status to clipboard".into(),
        ],
        "tray" => vec![
            "Ask for system status".into(),
            "Spawn a control".into(),
            "Clear the workspace".into(),
        ],
        _ => {
            let trimmed = text.trim();
            let mut items = vec![
                "Refresh agent status".into(),
                "Show display modes".into(),
                "List network interfaces".into(),
            ];
            if !trimmed.is_empty() {
                items.insert(0, short_label(trimmed, "Requested item"));
            }
            items
        }
    }
}

fn icon_name_for(text: &str) -> &'static str {
    let t = text.to_lowercase();
    if t.contains("network") || t.contains("wifi") {
        "network"
    } else if t.contains("power") || t.contains("battery") {
        "power"
    } else if t.contains("sound") || t.contains("audio") || t.contains("volume") {
        "audio"
    } else if t.contains("warn") || t.contains("alert") {
        "warning"
    } else {
        "info"
    }
}

fn media_src_for(text: &str) -> String {
    for token in text.split_whitespace() {
        if token.starts_with("http://")
            || token.starts_with("https://")
            || token.starts_with("file://")
            || token.starts_with('/')
        {
            return token
                .trim_matches(|c: char| c == '"' || c == '\'')
                .to_string();
        }
    }
    String::new()
}

/// Insert one primitive into `#ui.workspace`, plus an activity note.
pub fn spawn_plan(text: &str, seq: u64) -> Vec<PlanStep> {
    let (primitive, surface) = spawn_target(text);
    let id = spawn_id(primitive, seq);
    let (node, extra) = spawn_node(primitive, &surface, text, &id);
    let mut ops = vec![json!({
        "op": "insert",
        "anchor": "ui.workspace",
        "position": "child",
        "node": node,
    })];
    ops.extend(extra);
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: json!({ "ops": ops }),
        },
        PlanStep {
            action: "state.set".into(),
            params: json!({ "path": "task.spawn_seq", "value": seq + 1 }),
        },
        PlanStep {
            action: "state.patch".into(),
            params: json!({
                "ops": [{
                    "path": format!("task.workspace.{}", id.replace('.', "_")),
                    "value": { "kind": primitive, "surface": surface, "id": id },
                }]
            }),
        },
        activity_plan(&format!(
            "Spawned {surface} ({primitive}) as {id} in workspace"
        )),
    ]
}

/// Replace an existing workspace control in place (idempotent `ui.patch` insert).
pub fn respawn_plan(text: &str, existing_id: &str) -> Vec<PlanStep> {
    let (primitive, surface) = spawn_target(text);
    let (node, extra) = spawn_node(primitive, &surface, text, existing_id);
    let mut ops = vec![json!({
        "op": "insert",
        "anchor": "ui.workspace",
        "position": "child",
        "node": node,
    })];
    ops.extend(extra);
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: json!({ "ops": ops }),
        },
        activity_plan(&format!(
            "Replaced {existing_id} with {surface} ({primitive})"
        )),
    ]
}

/// Remove every agent-placed control from `#ui.workspace`.
pub fn clear_plan() -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.workspace.clear".into(),
            params: json!({ "anchor": "ui.workspace", "keep_hint": true }),
        },
        PlanStep {
            action: "state.set".into(),
            params: json!({ "path": "task.workspace", "value": {} }),
        },
        activity_plan("Workspace cleared — agent controls removed"),
    ]
}

/// Clear then spawn: "replace the workspace with a chart".
pub fn replace_workspace_plan(text: &str, seq: u64) -> Vec<PlanStep> {
    let mut plan = vec![PlanStep {
        action: "ui.workspace.clear".into(),
        params: json!({ "anchor": "ui.workspace", "keep_hint": false }),
    }];
    plan.extend(spawn_plan(text, seq));
    plan
}

/// Extract an MCP method name from "bind the button to calc.run.7".
pub fn bind_target_from_text(text: &str) -> Option<String> {
    let mut best: Option<String> = None;
    for raw in text.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
        let token =
            raw.trim_matches(|c: char| !(c.is_alphanumeric() || c == '.' || c == '_' || c == '-'));
        if token.len() < 3 || !token.contains('.') {
            continue;
        }
        if token.starts_with("ui.") && !KNOWN_BINDABLE.contains(&token) {
            continue;
        }
        let segments: Vec<&str> = token.split('.').collect();
        if segments.len() < 2 || segments.iter().any(|s| s.is_empty()) {
            continue;
        }
        if !segments[0]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            continue;
        }
        if best.as_ref().map(|b| token.len() > b.len()).unwrap_or(true) {
            best = Some(token.to_string());
        }
    }
    best
}

/// Bind a freshly spawned control to a concrete MCP method, synthesising a
/// lambda handler first when the method is not already on the bus.
pub fn bind_plan(text: &str, seq: u64) -> Vec<PlanStep> {
    let target = bind_target_from_text(text).unwrap_or_else(|| "agent.status".into());
    let (primitive, surface) = spawn_target(text);
    let id = spawn_id(primitive, seq);
    let namespace = target.split('.').next().unwrap_or("agent").to_string();
    let mut plan = Vec::new();

    if !KNOWN_BINDABLE.contains(&target.as_str()) {
        plan.push(PlanStep {
            action: "lambda.register".into(),
            params: json!({
                "manifest": {
                    "name": target,
                    "description": format!("Agent-synthesised handler for {target}"),
                    "source": synth_handler_source(),
                    "language": "python",
                    "entrypoint": "",
                    "capabilities": [],
                    "exposes_mcp": [format!("{namespace}.*")],
                }
            }),
        });
    }

    let (mut node, extra) = spawn_node(primitive, &surface, text, &id);
    if let Some(obj) = node.as_object_mut() {
        obj.insert(
            "bindings".into(),
            json!([{ "type": "mcp", "target": target, "event": "press" }]),
        );
    }
    let mut ops = vec![json!({
        "op": "insert",
        "anchor": "ui.workspace",
        "position": "child",
        "node": node,
    })];
    ops.extend(extra);
    plan.push(PlanStep {
        action: "ui.patch".into(),
        params: json!({ "ops": ops }),
    });
    plan.push(PlanStep {
        action: "ui.bind".into(),
        params: json!({
            "id": id,
            "binding": { "type": "mcp", "target": target },
        }),
    });
    plan.push(PlanStep {
        action: "state.set".into(),
        params: json!({ "path": "task.spawn_seq", "value": seq + 1 }),
    });
    plan.push(activity_plan(&format!("Bound {id} → {target}")));
    plan
}

fn synth_handler_source() -> String {
    r#"#!/usr/bin/env python3
import json, sys
data = json.loads(sys.stdin.read() or '{}')
print(json.dumps({"ok": True, "echo": data}))
"#
    .to_string()
}

/// Methods the multi-step planner is willing to chain without a model.
const PLANNABLE: [&str; 8] = [
    "agent.status",
    "state.set",
    "state.patch",
    "lambda.register",
    "event.publish",
    "ui.patch",
    "clipboard.get",
    "clipboard.set",
];

/// Parse "involving state.set × steps=4" into (method, step count).
pub fn multi_step_request(text: &str) -> Option<(String, usize)> {
    let lowered = text.to_lowercase();
    let method = PLANNABLE
        .iter()
        .find(|m| lowered.contains(&m.to_lowercase()))
        .map(|m| m.to_string())?;
    let steps = parse_step_count(&lowered).unwrap_or(3).clamp(1, 12);
    Some((method, steps))
}

fn parse_step_count(lowered: &str) -> Option<usize> {
    for marker in ["steps=", "steps ", "step=", "× ", "x "] {
        if let Some(pos) = lowered.find(marker) {
            let rest = &lowered[pos + marker.len()..];
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse::<usize>() {
                if n > 0 {
                    return Some(n);
                }
            }
        }
    }
    None
}

/// Build a real N-step MCP plan around `method` without needing a model.
pub fn multi_step_plan(method: &str, steps: usize, text: &str) -> Vec<PlanStep> {
    let mut plan = Vec::new();
    plan.push(PlanStep {
        action: "state.set".into(),
        params: json!({
            "path": "task.plan",
            "value": { "method": method, "steps": steps, "request": text.trim() },
        }),
    });
    for step in 0..steps {
        plan.push(step_for(method, step, text));
    }
    plan.push(PlanStep {
        action: "ui.patch".into(),
        params: json!({
            "ops": [{
                "op": "insert",
                "anchor": "ui.workspace",
                "position": "child",
                "node": {
                    "id": "ui.agent_plan_summary",
                    "type": "list",
                    "props": {
                        "label": format!("Plan: {method} × {steps}"),
                        "items": (0..steps)
                            .map(|n| format!("step {} — {method}", n + 1))
                            .collect::<Vec<_>>(),
                        "scroll_y": 0,
                    },
                    "bindings": [{ "type": "mcp", "target": "agent.status", "event": "activate" }],
                    "children": [],
                },
            }]
        }),
    });
    plan.push(activity_plan(&format!(
        "Executed {steps}-step plan around {method}"
    )));
    plan
}

fn step_for(method: &str, step: usize, text: &str) -> PlanStep {
    let n = step + 1;
    match method {
        "agent.status" => PlanStep {
            action: "agent.status".into(),
            params: json!({}),
        },
        "state.set" => PlanStep {
            action: "state.set".into(),
            params: json!({ "path": format!("task.plan.step_{n}"), "value": { "at": n, "request": text.trim() } }),
        },
        "state.patch" => PlanStep {
            action: "state.patch".into(),
            params: json!({ "ops": [{ "path": format!("task.plan.patch_{n}"), "value": n }] }),
        },
        "lambda.register" => PlanStep {
            action: "lambda.register".into(),
            params: json!({
                "manifest": {
                    "name": format!("plan.step_{n}"),
                    "description": format!("Plan step {n} for: {}", text.trim()),
                    "source": synth_handler_source(),
                    "language": "python",
                    "entrypoint": "",
                    "capabilities": [],
                    "exposes_mcp": ["plan.*"],
                }
            }),
        },
        "event.publish" => PlanStep {
            action: "event.publish".into(),
            params: json!({
                "category": "agent",
                "pattern": "plan.step",
                "payload": { "step": n, "request": text.trim() },
            }),
        },
        "clipboard.get" => PlanStep {
            action: "clipboard.get".into(),
            params: json!({}),
        },
        "clipboard.set" => PlanStep {
            action: "clipboard.set".into(),
            params: json!({ "text": format!("plan step {n}: {}", text.trim()) }),
        },
        // "ui.patch" and anything else paint progress into the workspace.
        _ => PlanStep {
            action: "ui.patch".into(),
            params: json!({
                "ops": [{
                    "op": "insert",
                    "anchor": "ui.workspace",
                    "position": "child",
                    "node": {
                        "id": format!("ui.agent_plan_step_{n}"),
                        "type": "text",
                        "props": { "role": "caption", "text": format!("step {n}: {}", text.trim()) },
                        "children": [],
                    },
                }]
            }),
        },
    }
}

/// Which system-daemon domain a request is about.
pub fn system_domain(text: &str) -> Option<&'static str> {
    let t = text.to_lowercase();
    if t.contains("display")
        || t.contains("monitor mode")
        || t.contains("resolution")
        || t.contains("brightness")
    {
        Some("display")
    } else if t.contains("net")
        || t.contains("interface")
        || t.contains("wifi")
        || t.contains("ethernet")
    {
        Some("net")
    } else if t.contains("audio") || t.contains("sound") || t.contains("volume") {
        Some("audio")
    } else if t.contains("power") || t.contains("battery") || t.contains("profile") {
        Some("power")
    } else {
        None
    }
}

/// Read-only MCP method for a domain (mutations need a policy grant, so the
/// conversational path only reads and then offers a confirmable control).
fn system_read_method(domain: &str) -> &'static str {
    match domain {
        "display" => "display.get_modes",
        "net" => "net.list_interfaces",
        "audio" => "audio.list_devices",
        _ => "power.get_profile",
    }
}

fn system_write_method(domain: &str) -> &'static str {
    match domain {
        "display" => "display.set_mode",
        "net" => "net.set_interface_state",
        "audio" => "audio.set_default",
        _ => "power.set_profile",
    }
}

/// Interface name mentioned in the request (`iface=eth2`, "eth0", "wlan0").
pub fn iface_from_text(text: &str) -> Option<String> {
    let lowered = text.to_lowercase();
    if let Some(pos) = lowered.find("iface=") {
        let rest = &lowered[pos + 6..];
        let name: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == ':' || *c == '.')
            .collect();
        if !name.is_empty() {
            return Some(name);
        }
    }
    for token in lowered.split(|c: char| !(c.is_ascii_alphanumeric())) {
        if (token.starts_with("eth") || token.starts_with("wlan") || token.starts_with("enp"))
            && token.len() > 3
        {
            return Some(token.to_string());
        }
    }
    None
}

/// Read a system domain over MCP, then paint the result plus a *confirmable*
/// mutation control. The mutation itself still goes through policy + broker
/// confirmation; the agent never forges a grant.
pub fn system_plan(text: &str) -> Vec<PlanStep> {
    let domain = system_domain(text).unwrap_or("power");
    let read = system_read_method(domain);
    let write = system_write_method(domain);
    let iface = iface_from_text(text);
    let panel_id = format!("ui.agent_system_{domain}");
    let mut read_params = json!({});
    if let (Some(name), true) = (iface.as_ref(), domain == "net") {
        read_params = json!({ "name": name });
    }
    vec![
        PlanStep {
            action: read.into(),
            params: read_params,
        },
        PlanStep {
            action: "ui.patch".into(),
            params: json!({
                "ops": [
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "position": "child",
                        "node": {
                            "id": panel_id,
                            "type": "list",
                            "props": {
                                "label": format!("{domain} — system-daemon"),
                                "items": [
                                    format!("read: {read}"),
                                    format!("mutate: {write} (needs confirmation)"),
                                    match &iface {
                                        Some(i) => format!("target: {i}"),
                                        None => "target: default device".to_string(),
                                    },
                                ],
                                "scroll_y": 0,
                            },
                            "bindings": [{ "type": "mcp", "target": read, "event": "activate" }],
                            "children": [],
                        },
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "position": "child",
                        "node": {
                            "id": format!("{panel_id}_refresh"),
                            "type": "button",
                            "props": { "label": format!("Refresh {domain}"), "variant": "primary" },
                            "bindings": [{ "type": "mcp", "target": read, "event": "press" }],
                            "children": [],
                        },
                    },
                ]
            }),
        },
        PlanStep {
            action: "state.patch".into(),
            params: json!({
                "ops": [{
                    "path": format!("task.system.{domain}"),
                    "value": { "read": read, "write": write, "iface": iface },
                }]
            }),
        },
        activity_plan(&format!(
            "Read {domain} via {read}; mutation requires confirmation"
        )),
    ]
}

/// Monitorable event streams the boot path actually publishes.
pub fn monitor_subject(text: &str) -> (&'static str, &'static str, &'static str) {
    let t = text.to_lowercase();
    if t.contains("netlink") || t.contains("network") {
        ("netlink", "system", "net.*")
    } else if t.contains("battery") || t.contains("power") {
        ("battery", "system", "power.*")
    } else if t.contains("hotplug") || t.contains("usb") || t.contains("device") {
        ("hotplug", "system", "device.*")
    } else if t.contains("audio") {
        ("audio", "system", "audio.*")
    } else if t.contains("display") {
        ("display", "system", "display.*")
    } else {
        ("session", "*", "*")
    }
}

/// Durable operator console: subscribe to the stream and paint a live panel.
pub fn monitor_plan(text: &str) -> Vec<PlanStep> {
    let (subject, category, pattern) = monitor_subject(text);
    let panel_id = format!("ui.agent_monitor_{subject}");
    vec![
        PlanStep {
            action: "event.subscribe".into(),
            params: json!({
                "category": category,
                "pattern": pattern,
                "subscriber": "agent-core",
            }),
        },
        PlanStep {
            action: "ui.patch".into(),
            params: json!({
                "ops": [
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "position": "child",
                        "node": {
                            "id": panel_id,
                            "type": "list",
                            "props": {
                                "label": format!("Monitor — {subject}"),
                                "items": [
                                    format!("subscribed: {category}/{pattern}"),
                                    "counters: system-daemon.stats".to_string(),
                                    "events land in task.monitor".to_string(),
                                ],
                                "scroll_y": 0,
                                "live": "polite",
                            },
                            "bindings": [{ "type": "mcp", "target": "system-daemon.stats", "event": "activate" }],
                            "children": [],
                        },
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "position": "child",
                        "node": {
                            "id": format!("{panel_id}_poll"),
                            "type": "button",
                            "props": { "label": format!("Poll {subject}"), "variant": "primary" },
                            "bindings": [{ "type": "mcp", "target": "system-daemon.stats", "event": "press" }],
                            "children": [],
                        },
                    },
                ]
            }),
        },
        PlanStep {
            action: "system-daemon.stats".into(),
            params: json!({}),
        },
        PlanStep {
            action: "state.patch".into(),
            params: json!({
                "ops": [{
                    "path": format!("task.monitor.{subject}"),
                    "value": { "category": category, "pattern": pattern, "active": true },
                }]
            }),
        },
        activity_plan(&format!("Monitoring {subject} ({category}/{pattern})")),
    ]
}

/// Onboarding tips shown by the product tour.
pub const TOUR_TIPS: [&str; 6] = [
    "Type a question in the chat field and press Enter or Send.",
    "Ask “show status” to place a status panel in the workspace.",
    "Ask “spawn a chart” (or toggle, slider, list, dialog, grid…) to build controls.",
    "Ask “clear workspace” to remove every agent-placed control.",
    "Ask about display, network, audio or power to read system state.",
    "Press Tab to move focus, Escape to dismiss a dialog, Ctrl+C/V to copy and paste.",
];

/// Product tour: real onboarding content with next/again controls.
pub fn tour_plan(step: usize) -> Vec<PlanStep> {
    let idx = step % TOUR_TIPS.len();
    let tip = TOUR_TIPS[idx];
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: json!({
                "ops": [
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "position": "child",
                        "node": {
                            "id": "ui.agent_tour",
                            "type": "list",
                            "props": {
                                "label": format!("Getting started ({}/{})", idx + 1, TOUR_TIPS.len()),
                                "items": TOUR_TIPS.to_vec(),
                                "selected": idx,
                                "scroll_y": 0,
                                "live": "polite",
                            },
                            "bindings": [{ "type": "mcp", "target": "agent.tour.next", "event": "activate" }],
                            "children": [],
                        },
                    },
                    {
                        "op": "insert",
                        "anchor": "ui.workspace",
                        "position": "child",
                        "node": {
                            "id": "ui.agent_tour_next",
                            "type": "button",
                            "props": { "label": "Next tip", "variant": "primary" },
                            "bindings": [{ "type": "mcp", "target": "agent.tour.next", "event": "press" }],
                            "children": [],
                        },
                    },
                    {
                        "op": "update",
                        "id": "ui.workspace_hint",
                        "props": { "text": tip },
                    },
                ]
            }),
        },
        PlanStep {
            action: "state.set".into(),
            params: json!({ "path": "task.tour_step", "value": idx + 1 }),
        },
        activity_plan(&format!("Tour tip {}/{}", idx + 1, TOUR_TIPS.len())),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_kind(plan: &[PlanStep]) -> String {
        let patch = plan
            .iter()
            .find(|s| s.action == "ui.patch")
            .expect("ui.patch");
        patch.params["ops"][0]["node"]["type"]
            .as_str()
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn every_primitive_has_a_spawn_path() {
        for (seq, primitive) in PRIMITIVES.iter().enumerate() {
            let plan = spawn_plan(&format!("spawn a {primitive} control"), seq as u64);
            assert_eq!(&node_kind(&plan), primitive, "primitive {primitive}");
        }
    }

    #[test]
    fn surface_aliases_map_onto_primitives() {
        for (alias, primitive) in SURFACE_ALIASES {
            let (resolved, surface) = spawn_target(&format!("spawn {alias} please"));
            assert_eq!(resolved, primitive, "alias {alias}");
            assert_eq!(surface, alias);
            let plan = spawn_plan(&format!("spawn {alias}"), 1);
            assert_eq!(&node_kind(&plan), primitive);
        }
    }

    #[test]
    fn spawn_ids_are_unique_per_sequence() {
        let a = spawn_plan("spawn a button", 1);
        let b = spawn_plan("spawn a button", 2);
        let id_a = a.iter().find(|s| s.action == "ui.patch").unwrap().params["ops"][0]["node"]
            ["id"]
            .as_str()
            .unwrap()
            .to_string();
        let id_b = b.iter().find(|s| s.action == "ui.patch").unwrap().params["ops"][0]["node"]
            ["id"]
            .as_str()
            .unwrap()
            .to_string();
        assert_ne!(id_a, id_b);
        assert!(a.iter().any(|s| s.action == "state.set"));
    }

    #[test]
    fn dialog_spawn_includes_dismiss_child() {
        let plan = spawn_plan("open a dialog about updates", 3);
        let patch = plan.iter().find(|s| s.action == "ui.patch").unwrap();
        let ops = patch.params["ops"].as_array().unwrap();
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[1]["node"]["type"], "button");
    }

    #[test]
    fn grid_spawn_includes_cells() {
        let plan = spawn_plan("spawn a grid", 4);
        let ops = plan.iter().find(|s| s.action == "ui.patch").unwrap().params["ops"]
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(ops[0]["node"]["type"], "grid");
        assert_eq!(ops.len(), 5);
    }

    #[test]
    fn clear_plan_calls_workspace_clear() {
        let plan = clear_plan();
        assert!(plan.iter().any(|s| s.action == "ui.workspace.clear"));
    }

    #[test]
    fn replace_workspace_clears_then_spawns() {
        let plan = replace_workspace_plan("replace workspace with a chart", 9);
        assert_eq!(plan[0].action, "ui.workspace.clear");
        assert_eq!(plan[0].params["keep_hint"], false);
        assert!(plan.iter().any(|s| s.action == "ui.patch"));
    }

    #[test]
    fn respawn_reuses_existing_id() {
        let plan = respawn_plan("a toggle", "ui.agent_button");
        let id = plan.iter().find(|s| s.action == "ui.patch").unwrap().params["ops"][0]["node"]
            ["id"]
            .as_str()
            .unwrap()
            .to_string();
        assert_eq!(id, "ui.agent_button");
    }

    #[test]
    fn bind_target_extracts_dotted_method() {
        assert_eq!(
            bind_target_from_text("bind button to calc.run.7").as_deref(),
            Some("calc.run.7")
        );
        assert_eq!(
            bind_target_from_text("bind it to agent.status").as_deref(),
            Some("agent.status")
        );
        assert!(bind_target_from_text("bind the button").is_none());
    }

    #[test]
    fn bind_plan_registers_lambda_then_binds() {
        let plan = bind_plan("bind button to calc.run.7", 2);
        assert_eq!(plan[0].action, "lambda.register");
        let patch = plan.iter().find(|s| s.action == "ui.patch").unwrap();
        assert_eq!(
            patch.params["ops"][0]["node"]["bindings"][0]["target"],
            "calc.run.7"
        );
        assert!(plan.iter().any(|s| s.action == "ui.bind"));
    }

    #[test]
    fn bind_plan_skips_lambda_for_known_method() {
        let plan = bind_plan("bind button to agent.status", 2);
        assert!(plan.iter().all(|s| s.action != "lambda.register"));
    }

    #[test]
    fn multi_step_request_parses_method_and_count() {
        assert_eq!(
            multi_step_request("Request plan involving state.set × steps=4"),
            Some(("state.set".into(), 4))
        );
        assert_eq!(
            multi_step_request("plan involving agent.status steps=2"),
            Some(("agent.status".into(), 2))
        );
        assert!(multi_step_request("just chatting").is_none());
    }

    #[test]
    fn multi_step_plan_emits_requested_step_count() {
        let plan = multi_step_plan("agent.status", 5, "plan involving agent.status");
        assert_eq!(
            plan.iter().filter(|s| s.action == "agent.status").count(),
            5
        );
        assert!(plan.iter().any(|s| s.action == "ui.patch"));
    }

    #[test]
    fn multi_step_plan_covers_all_plannable_methods() {
        for method in PLANNABLE {
            let plan = multi_step_plan(method, 2, "probe");
            assert!(
                plan.iter().filter(|s| s.action == method).count() >= 2 || method == "ui.patch",
                "method {method}"
            );
        }
    }

    #[test]
    fn system_plan_reads_domain_and_offers_confirmable_write() {
        for (text, read) in [
            ("show display modes", "display.get_modes"),
            ("list network interfaces iface=eth2", "net.list_interfaces"),
            ("audio devices", "audio.list_devices"),
            ("power profile", "power.get_profile"),
        ] {
            let plan = system_plan(text);
            assert_eq!(plan[0].action, read, "text {text}");
            assert!(plan.iter().any(|s| s.action == "ui.patch"));
        }
    }

    #[test]
    fn iface_parsing_handles_explicit_and_bare_names() {
        assert_eq!(
            iface_from_text("variant 4 (iface=eth0)").as_deref(),
            Some("eth0")
        );
        assert_eq!(iface_from_text("bring up wlan0").as_deref(), Some("wlan0"));
        assert!(iface_from_text("no interface here").is_none());
    }

    #[test]
    fn monitor_plan_subscribes_and_paints_panel() {
        let plan = monitor_plan("watch netlink during session");
        assert_eq!(plan[0].action, "event.subscribe");
        assert!(plan.iter().any(|s| s.action == "system-daemon.stats"));
    }

    #[test]
    fn monitor_subjects_cover_documented_streams() {
        for (text, subject) in [
            ("netlink", "netlink"),
            ("battery", "battery"),
            ("hotplug", "hotplug"),
            ("audio", "audio"),
            ("display", "display"),
            ("something else", "session"),
        ] {
            assert_eq!(monitor_subject(text).0, subject, "text {text}");
        }
    }

    #[test]
    fn tour_plan_advances_and_wraps() {
        let first = tour_plan(0);
        assert!(first.iter().any(|s| s.action == "ui.patch"));
        let wrapped = tour_plan(TOUR_TIPS.len());
        let idx = wrapped
            .iter()
            .find(|s| s.action == "state.set")
            .unwrap()
            .params["value"]
            .as_u64()
            .unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn media_src_extracted_from_text() {
        let plan = spawn_plan("spawn media https://example.com/a.mp4", 1);
        let patch = plan.iter().find(|s| s.action == "ui.patch").unwrap();
        assert_eq!(
            patch.params["ops"][0]["node"]["props"]["src"],
            "https://example.com/a.mp4"
        );
    }
}
