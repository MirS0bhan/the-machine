//! Keyboard shortcut table for the boot shell.
//!
//! A chord (`Ctrl+Shift+K`) maps to an **action name** that `ui.event` knows how
//! to run. Actions that touch other components resolve to an MCP method, so a
//! user-installed chord cannot do anything the bus would not already allow —
//! policy still gates the call.

use std::collections::BTreeMap;
use std::sync::RwLock;

use serde_json::{json, Value};

static TABLE: RwLock<Option<BTreeMap<String, Shortcut>>> = RwLock::new(None);

#[derive(Clone, Debug, PartialEq)]
pub struct Shortcut {
    /// Internal action name handled by `ui.event`'s key path.
    pub action: String,
    /// Optional MCP method the action forwards to.
    pub method: Option<String>,
    pub description: String,
    /// False for chords shipped with the shell, true once a user overrides them.
    pub user_defined: bool,
}

/// Canonical chord spelling: sorted modifiers then the key, e.g. `Ctrl+Shift+Z`.
pub fn normalize_chord(chord: &str) -> String {
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut meta = false;
    let mut key = String::new();
    for part in chord
        .split(['+', '-'])
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "alt" | "option" => alt = true,
            "shift" => shift = true,
            "meta" | "super" | "cmd" | "command" | "win" => meta = true,
            _ => key = canonical_key(part),
        }
    }
    let mut out = String::new();
    if ctrl {
        out.push_str("Ctrl+");
    }
    if alt {
        out.push_str("Alt+");
    }
    if shift {
        out.push_str("Shift+");
    }
    if meta {
        out.push_str("Meta+");
    }
    out.push_str(&key);
    out
}

fn canonical_key(key: &str) -> String {
    let lowered = key.to_ascii_lowercase();
    match lowered.as_str() {
        "left" | "arrowleft" => "ArrowLeft".into(),
        "right" | "arrowright" => "ArrowRight".into(),
        "up" | "arrowup" => "ArrowUp".into(),
        "down" | "arrowdown" => "ArrowDown".into(),
        "esc" | "escape" => "Escape".into(),
        "return" | "enter" => "Enter".into(),
        "tab" => "Tab".into(),
        "space" | " " => "Space".into(),
        "pageup" | "prior" => "PageUp".into(),
        "pagedown" | "next" => "PageDown".into(),
        "home" => "Home".into(),
        "end" => "End".into(),
        "delete" | "del" => "Delete".into(),
        "backspace" | "back" => "Backspace".into(),
        "printscreen" | "print" | "prtsc" | "sysrq" => "PrintScreen".into(),
        "super_l" | "super_r" | "supermeta" => "Meta".into(),
        other if other.chars().count() == 1 => other.to_ascii_uppercase(),
        other => {
            let mut c = other.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        }
    }
}

fn shortcut(action: &str, method: Option<&str>, description: &str) -> Shortcut {
    Shortcut {
        action: action.to_string(),
        method: method.map(|m| m.to_string()),
        description: description.to_string(),
        user_defined: false,
    }
}

/// Chords the boot shell ships with. Every one is implemented by `ui.event`.
pub fn defaults() -> BTreeMap<String, Shortcut> {
    let mut m = BTreeMap::new();
    let mut add = |chord: &str, s: Shortcut| {
        m.insert(normalize_chord(chord), s);
    };
    add("Tab", shortcut("focus.next", None, "Move focus forward"));
    add(
        "Shift+Tab",
        shortcut("focus.previous", None, "Move focus backward"),
    );
    add(
        "Alt+Tab",
        shortcut(
            "surface.cycle",
            None,
            "Cycle focus between top-level surfaces",
        ),
    );
    add(
        "Alt+Shift+Tab",
        shortcut(
            "surface.cycle.reverse",
            None,
            "Cycle surfaces in reverse order",
        ),
    );
    add(
        "Escape",
        shortcut(
            "dismiss",
            None,
            "Cancel IME preedit, else dismiss the dialog",
        ),
    );
    add(
        "Enter",
        shortcut("activate", None, "Activate focused control"),
    );
    add(
        "Ctrl+C",
        shortcut("clipboard.copy", Some("clipboard.set"), "Copy selection"),
    );
    add(
        "Ctrl+X",
        shortcut("clipboard.cut", Some("clipboard.set"), "Cut selection"),
    );
    add(
        "Ctrl+V",
        shortcut("clipboard.paste", Some("clipboard.get"), "Paste clipboard"),
    );
    add("Ctrl+A", shortcut("select.all", None, "Select all text"));
    add("Ctrl+Z", shortcut("undo", None, "Undo last edit"));
    add("Ctrl+Shift+Z", shortcut("redo", None, "Redo last undo"));
    add("Ctrl+Y", shortcut("redo", None, "Redo last undo"));
    add(
        "Meta",
        shortcut("menu.open", None, "Open the agent command menu"),
    );
    add(
        "PrintScreen",
        shortcut("snapshot", None, "Capture the rendered tree snapshot"),
    );
    add(
        "PageUp",
        shortcut("scroll.page_up", None, "Scroll focused list up a page"),
    );
    add(
        "PageDown",
        shortcut("scroll.page_down", None, "Scroll focused list down a page"),
    );
    add(
        "Ctrl+L",
        shortcut(
            "workspace.clear",
            Some("ui.workspace.clear"),
            "Clear agent-placed workspace controls",
        ),
    );
    add(
        "Ctrl+Shift+A",
        shortcut(
            "a11y.tree",
            Some("ui.a11y.tree"),
            "Read the accessibility tree",
        ),
    );
    add(
        "Ctrl+Enter",
        shortcut(
            "chat.send",
            Some("agent.chat.send"),
            "Send the chat message",
        ),
    );
    m
}

fn with_table<T>(f: impl FnOnce(&mut BTreeMap<String, Shortcut>) -> T) -> T {
    let mut guard = TABLE.write().expect("shortcut table poisoned");
    let table = guard.get_or_insert_with(defaults);
    f(table)
}

/// Resolve a chord to its shortcut, honouring user overrides.
pub fn resolve(chord: &str) -> Option<Shortcut> {
    let key = normalize_chord(chord);
    with_table(|t| t.get(&key).cloned())
}

/// Build the chord string for a key event.
pub fn chord_for(key: &str, ctrl: bool, alt: bool, shift: bool, meta: bool) -> String {
    let mut parts = Vec::new();
    if ctrl {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    if meta {
        parts.push("Meta");
    }
    let joined = if parts.is_empty() {
        key.to_string()
    } else {
        format!("{}+{}", parts.join("+"), key)
    };
    normalize_chord(&joined)
}

/// Install or replace a chord. Returns the normalized chord on success.
pub fn set(
    chord: &str,
    action: &str,
    method: Option<&str>,
    description: &str,
) -> Result<String, String> {
    let key = normalize_chord(chord);
    if key.is_empty() {
        return Err("chord required".into());
    }
    if action.is_empty() {
        return Err("action required".into());
    }
    if !ACTIONS.contains(&action) {
        return Err(format!(
            "unknown action {action}; known actions: {}",
            ACTIONS.join(", ")
        ));
    }
    with_table(|t| {
        t.insert(
            key.clone(),
            Shortcut {
                action: action.to_string(),
                method: method.map(|m| m.to_string()),
                description: if description.is_empty() {
                    format!("User shortcut for {action}")
                } else {
                    description.to_string()
                },
                user_defined: true,
            },
        );
    });
    Ok(key)
}

/// Remove a chord (falls back to the default table entry if one exists).
pub fn unset(chord: &str) -> bool {
    let key = normalize_chord(chord);
    with_table(|t| {
        let removed = t.remove(&key).is_some();
        if let Some(default) = defaults().get(&key) {
            t.insert(key.clone(), default.clone());
        }
        removed
    })
}

pub fn reset() {
    let mut guard = TABLE.write().expect("shortcut table poisoned");
    *guard = Some(defaults());
}

/// The shortcut table is process-global, so tests that mutate it must not run
/// concurrently. Hold this guard for the duration of such a test.
#[cfg(test)]
pub fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    match LOCK.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Action names `ui.event` implements. Used to validate `ui.shortcuts.set`.
pub const ACTIONS: [&str; 18] = [
    "focus.next",
    "focus.previous",
    "surface.cycle",
    "surface.cycle.reverse",
    "dismiss",
    "activate",
    "clipboard.copy",
    "clipboard.cut",
    "clipboard.paste",
    "select.all",
    "undo",
    "redo",
    "menu.open",
    "snapshot",
    "scroll.page_up",
    "scroll.page_down",
    "workspace.clear",
    "a11y.tree",
];

pub fn list() -> Value {
    let entries: Vec<Value> = with_table(|t| {
        t.iter()
            .map(|(chord, s)| {
                json!({
                    "chord": chord,
                    "action": s.action,
                    "method": s.method,
                    "description": s.description,
                    "user_defined": s.user_defined,
                })
            })
            .collect()
    });
    json!({
        "shortcuts": entries,
        "actions": ACTIONS,
        "editable": true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_modifier_spelling_and_order() {
        assert_eq!(normalize_chord("shift+ctrl+z"), "Ctrl+Shift+Z");
        assert_eq!(normalize_chord("Super+L"), "Meta+L");
        assert_eq!(normalize_chord("cmd-c"), "Meta+C");
        assert_eq!(normalize_chord("prtsc"), "PrintScreen");
        assert_eq!(normalize_chord("arrowleft"), "ArrowLeft");
    }

    #[test]
    fn defaults_cover_every_documented_action() {
        let table = defaults();
        for action in ACTIONS {
            assert!(
                table.values().any(|s| s.action == action),
                "no default chord for {action}"
            );
        }
    }

    #[test]
    fn chord_for_matches_default_table() {
        let _g = test_guard();
        reset();
        assert_eq!(
            resolve(&chord_for("C", true, false, false, false))
                .map(|s| s.action)
                .as_deref(),
            Some("clipboard.copy")
        );
        assert_eq!(
            resolve(&chord_for("Tab", false, true, false, false))
                .map(|s| s.action)
                .as_deref(),
            Some("surface.cycle")
        );
        assert_eq!(
            resolve(&chord_for("Tab", false, false, true, false))
                .map(|s| s.action)
                .as_deref(),
            Some("focus.previous")
        );
    }

    #[test]
    fn user_shortcut_overrides_then_resets() {
        let _g = test_guard();
        reset();
        let chord = set(
            "Ctrl+Shift+K",
            "workspace.clear",
            Some("ui.workspace.clear"),
            "",
        )
        .expect("set");
        assert_eq!(chord, "Ctrl+Shift+K");
        let s = resolve("ctrl+shift+k").expect("resolved");
        assert_eq!(s.action, "workspace.clear");
        assert!(s.user_defined);
        reset();
        assert!(resolve("Ctrl+Shift+K").is_none());
    }

    #[test]
    fn user_shortcut_can_rebind_a_default_chord() {
        let _g = test_guard();
        reset();
        set("Ctrl+L", "snapshot", None, "screenshot instead").expect("set");
        assert_eq!(resolve("Ctrl+L").unwrap().action, "snapshot");
        assert!(unset("Ctrl+L"));
        // Unsetting restores the shipped default rather than leaving a hole.
        assert_eq!(resolve("Ctrl+L").unwrap().action, "workspace.clear");
        reset();
    }

    #[test]
    fn unknown_action_is_rejected() {
        let _g = test_guard();
        reset();
        assert!(set("Ctrl+Shift+Q", "launch.rocket", None, "").is_err());
    }

    #[test]
    fn list_reports_chords_and_actions() {
        let _g = test_guard();
        reset();
        let v = list();
        let count = v["shortcuts"].as_array().unwrap().len();
        assert!(count >= ACTIONS.len());
        assert_eq!(v["editable"], true);
    }
}
