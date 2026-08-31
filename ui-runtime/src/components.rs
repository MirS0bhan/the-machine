//! Boot-path component registry names (subset ported from ui-engine).

use serde_json::{json, Value};

/// Components known to the design system / Python registry that the boot path
/// can author with AUIL primitives (not a full recipe expander yet).
pub fn catalog() -> Vec<Value> {
    [
        ("Surface", "stack + Surface mixin tokens"),
        ("Card", "stack card chrome"),
        ("ListRow", "list row pattern"),
        ("PrimaryButton", "button variant=primary"),
        ("IconBtn", "button + icon"),
        ("Field", "field primitive"),
        ("MediaPlayer", "media primitive"),
        ("Chart", "chart primitive"),
        ("SessionGreeting", "boot.auil stack"),
        ("AlertDialog", "dialog + destructive confirm"),
        ("ConfirmDialog", "dialog soft exclusivity"),
        ("Toast", "transient text+button (agent-authored)"),
    ]
    .into_iter()
    .map(|(name, note)| {
        json!({
            "name": name,
            "status": if matches!(name, "SessionGreeting" | "PrimaryButton" | "Field" | "ConfirmDialog" | "MediaPlayer" | "Chart") {
                "boot-partial"
            } else {
                "specified"
            },
            "note": note,
        })
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_session_greeting() {
        let catalog = catalog();
        let names: Vec<_> = catalog
            .iter()
            .filter_map(|v| v.get("name").and_then(|n| n.as_str()))
            .collect();
        assert!(names.contains(&"SessionGreeting"));
        assert!(names.contains(&"PrimaryButton"));
    }
}
