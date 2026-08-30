//! Agent skills: built-ins + State Store overlay.

use crate::client::mcp_call;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Skill {
    pub name: String,
    #[serde(default)]
    pub version: u64,
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub description: String,
}

pub fn builtin_skills() -> Vec<Skill> {
    vec![
        Skill {
            name: "intent-classification".into(),
            version: 4,
            applies_to: vec!["category:input".into(), "category:*".into()],
            system_prompt: "Classify user input into intent, complexity, and routing.".into(),
            description: "Intent classifier".into(),
        },
        Skill {
            name: "media-control".into(),
            version: 1,
            applies_to: vec!["intent:media_control".into(), "intent:media.play".into()],
            system_prompt: "Route media intents to media_player lambda.".into(),
            description: "Media control".into(),
        },
        Skill {
            name: "calculator-synth".into(),
            version: 1,
            applies_to: vec!["intent:calculator".into(), "intent:calc.eval".into()],
            system_prompt: "Synthesize calc.eval lambda with Python source when missing.".into(),
            description: "Calculator synthesis".into(),
        },
        Skill {
            name: "notification-triage".into(),
            version: 1,
            applies_to: vec![
                "category:notification".into(),
                "intent:notification.triage".into(),
            ],
            system_prompt: "Summarize notifications and patch UI with actionable cards.".into(),
            description: "Notification triage".into(),
        },
    ]
}

pub async fn load_skills() -> Vec<Skill> {
    let mut skills = builtin_skills();
    if let Some(list) = mcp_call(
        "state.list",
        serde_json::json!({ "prefix": "agent.skills." }),
    )
    .await
    .and_then(|v| v.get("entries").cloned())
    {
        if let Some(entries) = list.as_array() {
            for entry in entries {
                if let Some(val) = entry.get("value") {
                    if let Ok(sk) = serde_json::from_value::<Skill>(val.clone()) {
                        skills.retain(|s| s.name != sk.name);
                        skills.push(sk);
                    }
                }
            }
        }
    }
    skills
}

pub fn skills_for_wake(skills: &[Skill], category: &str, intent: &str) -> Vec<Skill> {
    let cat_key = format!("category:{}", category);
    let intent_key = format!("intent:{}", intent);
    skills
        .iter()
        .filter(|s| {
            s.applies_to.is_empty()
                || s.applies_to
                    .iter()
                    .any(|a| a == "*" || a == &cat_key || a == &intent_key || a == "category:*")
        })
        .cloned()
        .collect()
}

pub fn build_skill_prompt(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return String::new();
    }
    let mut out = String::from("Active skills:\n");
    for s in skills {
        out.push_str(&format!(
            "- {} (v{}): {}\n",
            s.name, s.version, s.system_prompt
        ));
    }
    out
}

pub async fn seed_default_skills_if_empty() {
    let existing = mcp_call(
        "state.list",
        serde_json::json!({ "prefix": "agent.skills." }),
    )
    .await
    .and_then(|v| v.get("entries").and_then(|e| e.as_array()).map(|a| a.len()))
    .unwrap_or(0);
    if existing > 0 {
        return;
    }
    for sk in builtin_skills() {
        let path = format!("agent.skills.{}", sk.name);
        let _ = mcp_call(
            "state.set",
            serde_json::json!({ "path": path, "value": sk }),
        )
        .await;
    }
}
