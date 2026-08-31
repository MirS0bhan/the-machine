//! Structured multi-turn conversation state.
//!
//! `task.chat_turns` is the source of truth (a JSON array of turns persisted by
//! state-store, so it survives reboots); `#ui.chat_log` is a rendering of it.
//! Keeping the array authoritative is what makes edit / regenerate / pin /
//! export real instead of string surgery on a text blob.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::planner::{activity_plan, truncate_chat_log, PlanStep, CHAT_TURNS_MAX};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ChatTurn {
    /// Monotonic per-session turn number, used by edit / regenerate / pin.
    pub n: u64,
    pub user: String,
    pub assistant: String,
    /// Which backend produced `assistant`: `cloud`, `local`, or `heuristic`.
    #[serde(default)]
    pub route: String,
    #[serde(default)]
    pub at: String,
    #[serde(default)]
    pub pinned: bool,
    /// Attachment references (paths / URLs) supplied with the user message.
    #[serde(default)]
    pub attachments: Vec<String>,
    /// `text`, `voice`, or `hybrid` — mirrors the AUIL field `input-mode`.
    #[serde(default)]
    pub source: String,
}

impl ChatTurn {
    pub fn new(n: u64, user: &str, assistant: &str, route: &str) -> Self {
        ChatTurn {
            n,
            user: user.trim().to_string(),
            assistant: assistant.trim().to_string(),
            route: route.to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            pinned: false,
            attachments: Vec::new(),
            source: "text".into(),
        }
    }
}

/// Parse `task.chat_turns` into turns, tolerating a missing or malformed value.
pub fn parse_turns(value: &Value) -> Vec<ChatTurn> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|v| serde_json::from_value::<ChatTurn>(v.clone()).ok())
            .collect(),
        _ => Vec::new(),
    }
}

pub fn next_turn_number(turns: &[ChatTurn]) -> u64 {
    turns.iter().map(|t| t.n).max().unwrap_or(0) + 1
}

/// Append a turn, evicting the oldest **unpinned** turns when over the cap.
pub fn append_turn(turns: &[ChatTurn], turn: ChatTurn) -> Vec<ChatTurn> {
    let mut out: Vec<ChatTurn> = turns.to_vec();
    out.push(turn);
    while out.len() > CHAT_TURNS_MAX {
        match out.iter().position(|t| !t.pinned) {
            Some(idx) => {
                out.remove(idx);
            }
            // Everything is pinned: drop the oldest anyway rather than grow forever.
            None => {
                out.remove(0);
            }
        }
    }
    out
}

/// Render turns into the `#ui.chat_log` text, trimmed from the head when long.
pub fn render_log(turns: &[ChatTurn]) -> String {
    let mut lines = Vec::new();
    for turn in turns {
        let pin = if turn.pinned { "📌 " } else { "" };
        if !turn.user.is_empty() {
            if turn.attachments.is_empty() {
                lines.push(format!("{pin}You: {}", turn.user));
            } else {
                lines.push(format!(
                    "{pin}You: {} [{}]",
                    turn.user,
                    turn.attachments.join(", ")
                ));
            }
        }
        if !turn.assistant.is_empty() {
            lines.push(format!("Assistant: {}", turn.assistant));
        }
    }
    truncate_chat_log(&lines.join("\n"))
}

/// Plain-text transcript for `agent.chat.export` (never truncated).
pub fn export_transcript(turns: &[ChatTurn]) -> String {
    let mut out = Vec::new();
    for turn in turns {
        out.push(format!(
            "[{}] #{} ({}){}",
            turn.at,
            turn.n,
            if turn.route.is_empty() {
                "unknown"
            } else {
                &turn.route
            },
            if turn.pinned { " pinned" } else { "" }
        ));
        if !turn.user.is_empty() {
            out.push(format!("You: {}", turn.user));
        }
        if !turn.assistant.is_empty() {
            out.push(format!("Assistant: {}", turn.assistant));
        }
        out.push(String::new());
    }
    out.join("\n")
}

/// Suggested next prompts shown in `#ui.suggestions`.
pub fn suggestions(turns: &[ChatTurn]) -> Vec<String> {
    let last = turns
        .last()
        .map(|t| t.user.to_lowercase())
        .unwrap_or_default();
    if last.contains("status") {
        vec![
            "Spawn a chart of the last plan".into(),
            "List network interfaces".into(),
            "Clear the workspace".into(),
        ]
    } else if last.contains("spawn") || last.contains("workspace") {
        vec![
            "Clear the workspace".into(),
            "Bind the button to calc.run.1".into(),
            "Show a dialog".into(),
        ]
    } else if turns.is_empty() {
        vec![
            "What can you do?".into(),
            "Show status".into(),
            "Give me a tour".into(),
        ]
    } else {
        vec![
            "Show status".into(),
            "Spawn a list of session actions".into(),
            "Read the power profile".into(),
        ]
    }
}

/// Ops that keep chat chrome (`#ui.chat_log`, `#ui.suggestions`) in sync.
pub fn chrome_ops(turns: &[ChatTurn]) -> Vec<Value> {
    vec![
        json!({
            "op": "update",
            "id": "ui.chat_log",
            "props": { "text": render_log(turns), "turns": turns.len(), "live": "polite" }
        }),
        json!({
            "op": "update",
            "id": "ui.suggestions",
            "props": { "items": suggestions(turns), "label": "Suggestions" }
        }),
    ]
}

/// State ops persisting both the structured turns and the rendered log.
pub fn state_ops(turns: &[ChatTurn]) -> Vec<Value> {
    json_ops(turns)
}

fn json_ops(turns: &[ChatTurn]) -> Vec<Value> {
    vec![
        json!({ "path": "task.chat_turns", "value": serde_json::to_value(turns).unwrap_or(Value::Null) }),
        json!({ "path": "task.chat_log", "value": render_log(turns) }),
    ]
}

/// The canonical "one conversational turn happened" plan.
pub fn turn_plan(turns: &[ChatTurn]) -> Vec<PlanStep> {
    vec![
        PlanStep {
            action: "ui.patch".into(),
            params: json!({ "ops": chrome_ops(turns) }),
        },
        PlanStep {
            action: "state.patch".into(),
            params: json!({ "ops": state_ops(turns) }),
        },
    ]
}

/// Replace the user text of turn `n` (chat edit) and mark it for regeneration.
pub fn edit_turn(turns: &[ChatTurn], n: u64, new_text: &str) -> Option<Vec<ChatTurn>> {
    let mut out = turns.to_vec();
    let idx = out.iter().position(|t| t.n == n)?;
    out[idx].user = new_text.trim().to_string();
    out[idx].assistant.clear();
    out[idx].route = "pending".into();
    out[idx].at = chrono::Utc::now().to_rfc3339();
    // Editing a turn invalidates everything said after it.
    out.truncate(idx + 1);
    Some(out)
}

/// Drop the assistant side of a turn so the reply chain runs again.
pub fn clear_reply(turns: &[ChatTurn], n: u64) -> Option<Vec<ChatTurn>> {
    let mut out = turns.to_vec();
    let idx = out.iter().position(|t| t.n == n)?;
    out[idx].assistant.clear();
    out[idx].route = "pending".into();
    Some(out)
}

pub fn set_reply(turns: &[ChatTurn], n: u64, reply: &str, route: &str) -> Option<Vec<ChatTurn>> {
    let mut out = turns.to_vec();
    let idx = out.iter().position(|t| t.n == n)?;
    out[idx].assistant = reply.trim().to_string();
    out[idx].route = route.to_string();
    out[idx].at = chrono::Utc::now().to_rfc3339();
    Some(out)
}

pub fn set_pinned(turns: &[ChatTurn], n: u64, pinned: bool) -> Option<Vec<ChatTurn>> {
    let mut out = turns.to_vec();
    let idx = out.iter().position(|t| t.n == n)?;
    out[idx].pinned = pinned;
    Some(out)
}

/// Undo: remove the most recent turn (used by chat undo / "take that back").
pub fn undo_last(turns: &[ChatTurn]) -> Vec<ChatTurn> {
    let mut out = turns.to_vec();
    out.pop();
    out
}

/// Migrate a legacy `task.chat_log` blob into structured turns so sessions that
/// booted before turns existed keep their history.
pub fn turns_from_legacy_log(log: &str) -> Vec<ChatTurn> {
    let mut turns: Vec<ChatTurn> = Vec::new();
    let mut n = 0u64;
    for line in log.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("You: ") {
            n += 1;
            turns.push(ChatTurn {
                n,
                user: rest.to_string(),
                assistant: String::new(),
                route: "restored".into(),
                at: String::new(),
                pinned: false,
                attachments: Vec::new(),
                source: "text".into(),
            });
        } else if let Some(rest) = line.strip_prefix("Assistant: ") {
            match turns.last_mut() {
                Some(turn) if turn.assistant.is_empty() => turn.assistant = rest.to_string(),
                _ => {
                    n += 1;
                    turns.push(ChatTurn {
                        n,
                        user: String::new(),
                        assistant: rest.to_string(),
                        route: "restored".into(),
                        at: String::new(),
                        pinned: false,
                        attachments: Vec::new(),
                        source: "text".into(),
                    });
                }
            }
        }
    }
    turns
}

/// Attachment / voice metadata extracted from an `agent.chat.send` payload.
pub fn attachments_from_payload(payload: &Value) -> Vec<String> {
    let mut out = Vec::new();
    for key in ["attachments", "attachment", "files"] {
        match payload.get(key) {
            Some(Value::Array(items)) => {
                for item in items {
                    if let Some(s) = item.as_str() {
                        out.push(s.to_string());
                    }
                }
            }
            Some(Value::String(s)) if !s.is_empty() => out.push(s.clone()),
            _ => {}
        }
    }
    out
}

pub fn source_from_payload(payload: &Value) -> String {
    payload
        .get("input_mode")
        .or_else(|| payload.get("source_mode"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            if payload
                .get("voice")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                "voice".into()
            } else {
                "text".into()
            }
        })
}

/// Activity chrome describing which backend answered.
pub fn route_activity(route: &str) -> PlanStep {
    let label = match route {
        "cloud" => "Replied via cloud model",
        "local" => "Replied via local model",
        "heuristic" => "Replied locally (no model backend configured)",
        "pending" => "Waiting for a reply backend",
        other => other,
    };
    activity_plan(label)
}

/// A `@skill` mention in the user text, e.g. "@calculator 2+2".
pub fn skill_mention(text: &str) -> Option<String> {
    for token in text.split_whitespace() {
        if let Some(name) = token.strip_prefix('@') {
            let cleaned: String = name
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-' || *c == '.')
                .collect();
            if cleaned.len() >= 2 {
                return Some(cleaned);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turns(n: usize) -> Vec<ChatTurn> {
        (1..=n)
            .map(|i| ChatTurn::new(i as u64, &format!("q{i}"), &format!("a{i}"), "heuristic"))
            .collect()
    }

    #[test]
    fn append_assigns_next_number() {
        let t = turns(2);
        assert_eq!(next_turn_number(&t), 3);
        let t = append_turn(&t, ChatTurn::new(3, "q3", "a3", "cloud"));
        assert_eq!(t.len(), 3);
        assert_eq!(t[2].route, "cloud");
    }

    #[test]
    fn render_log_keeps_all_turns_in_order() {
        let log = render_log(&turns(3));
        assert!(log.contains("You: q1"));
        assert!(log.contains("Assistant: a3"));
        assert!(log.find("q1").unwrap() < log.find("q3").unwrap());
    }

    #[test]
    fn cap_evicts_unpinned_first() {
        let mut t = turns(CHAT_TURNS_MAX);
        t[0].pinned = true;
        let t = append_turn(&t, ChatTurn::new(999, "new", "reply", "local"));
        assert_eq!(t.len(), CHAT_TURNS_MAX);
        assert!(t.iter().any(|x| x.n == 1), "pinned turn must survive");
        assert!(t.iter().any(|x| x.n == 999));
    }

    #[test]
    fn edit_truncates_following_turns() {
        let t = turns(4);
        let edited = edit_turn(&t, 2, "changed").expect("turn 2");
        assert_eq!(edited.len(), 2);
        assert_eq!(edited[1].user, "changed");
        assert!(edited[1].assistant.is_empty());
    }

    #[test]
    fn regenerate_clears_then_sets_reply() {
        let t = turns(2);
        let cleared = clear_reply(&t, 2).unwrap();
        assert!(cleared[1].assistant.is_empty());
        let set = set_reply(&cleared, 2, "fresh", "cloud").unwrap();
        assert_eq!(set[1].assistant, "fresh");
        assert_eq!(set[1].route, "cloud");
    }

    #[test]
    fn pin_and_unpin_roundtrip() {
        let t = turns(1);
        let pinned = set_pinned(&t, 1, true).unwrap();
        assert!(pinned[0].pinned);
        assert!(render_log(&pinned).contains("📌"));
        let unpinned = set_pinned(&pinned, 1, false).unwrap();
        assert!(!unpinned[0].pinned);
    }

    #[test]
    fn undo_removes_last_turn() {
        let t = undo_last(&turns(3));
        assert_eq!(t.len(), 2);
    }

    #[test]
    fn export_includes_routes_and_timestamps() {
        let out = export_transcript(&turns(2));
        assert!(out.contains("#1"));
        assert!(out.contains("heuristic"));
        assert!(out.contains("You: q1"));
    }

    #[test]
    fn legacy_log_migrates_to_turns() {
        let t = turns_from_legacy_log("You: hi\nAssistant: hello\nYou: again\nAssistant: yes");
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].user, "hi");
        assert_eq!(t[1].assistant, "yes");
    }

    #[test]
    fn legacy_log_handles_assistant_only_greeting() {
        let t = turns_from_legacy_log("Assistant: Welcome aboard.");
        assert_eq!(t.len(), 1);
        assert!(t[0].user.is_empty());
        assert_eq!(t[0].assistant, "Welcome aboard.");
    }

    #[test]
    fn attachments_and_voice_source_parsed() {
        let payload = json!({ "attachments": ["/tmp/a.png"], "voice": true });
        assert_eq!(attachments_from_payload(&payload), vec!["/tmp/a.png"]);
        assert_eq!(source_from_payload(&payload), "voice");
        let payload = json!({ "attachment": "/tmp/b.txt" });
        assert_eq!(attachments_from_payload(&payload), vec!["/tmp/b.txt"]);
        assert_eq!(source_from_payload(&json!({})), "text");
    }

    #[test]
    fn attachments_render_in_log() {
        let mut t = ChatTurn::new(1, "look at this", "sure", "local");
        t.attachments = vec!["/tmp/a.png".into()];
        assert!(render_log(&[t]).contains("/tmp/a.png"));
    }

    #[test]
    fn suggestions_react_to_last_turn() {
        assert!(suggestions(&[])
            .iter()
            .any(|s| s.contains("What can you do")));
        let t = vec![ChatTurn::new(1, "show status", "ok", "local")];
        assert!(suggestions(&t).iter().any(|s| s.contains("network")));
    }

    #[test]
    fn chrome_ops_update_log_and_suggestions() {
        let ops = chrome_ops(&turns(1));
        assert_eq!(ops[0]["id"], "ui.chat_log");
        assert_eq!(ops[1]["id"], "ui.suggestions");
    }

    #[test]
    fn skill_mentions_detected() {
        assert_eq!(
            skill_mention("@calculator 2+2").as_deref(),
            Some("calculator")
        );
        assert!(skill_mention("plain text").is_none());
    }

    #[test]
    fn turn_plan_patches_ui_and_state() {
        let plan = turn_plan(&turns(1));
        assert_eq!(plan[0].action, "ui.patch");
        assert_eq!(plan[1].action, "state.patch");
        let paths: Vec<&str> = plan[1].params["ops"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| o["path"].as_str().unwrap())
            .collect();
        assert!(paths.contains(&"task.chat_turns"));
        assert!(paths.contains(&"task.chat_log"));
    }
}
