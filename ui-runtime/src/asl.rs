//! Minimal ASL subset for the Rust boot path.
//!
//! Parses `token` / `scale` declarations enough to seed themes and resolve
//! `token:category.role` references. Full mixin/`style` blocks remain in
//! `ui-engine/asl_parser.py` until ported.

use std::collections::HashMap;

use serde_json::{json, Value};

#[derive(Debug, Default, Clone)]
pub struct AslDocument {
    pub tokens: HashMap<String, String>,
    pub scales: HashMap<String, HashMap<String, u32>>,
}

/// Parse a subset of ASL source: `token name = value` and `scale name: k=v ...`.
pub fn parse_asl(source: &str) -> AslDocument {
    let mut doc = AslDocument::default();
    for raw in source.lines() {
        let line = raw.split("//").next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("token ") {
            if let Some((name, value)) = rest.split_once('=') {
                doc.tokens
                    .insert(name.trim().to_string(), strip_quotes(value.trim()));
            }
        } else if let Some(rest) = line.strip_prefix("scale ") {
            if let Some((name, body)) = rest.split_once(':') {
                let mut tiers = HashMap::new();
                for tok in body.split_whitespace() {
                    if let Some((k, v)) = tok.split_once('=') {
                        if let Ok(n) = v.parse::<u32>() {
                            tiers.insert(k.to_string(), n);
                        }
                    }
                }
                doc.scales.insert(name.trim().to_string(), tiers);
            }
        }
    }
    doc
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else if s.len() >= 2 && s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub fn resolve_ref(doc: &AslDocument, value: &str) -> Option<Value> {
    if let Some(name) = value.strip_prefix("token:") {
        return doc.tokens.get(name).map(|v| json!(v));
    }
    if let Some(rest) = value.strip_prefix("r-") {
        return doc
            .scales
            .get("radius")
            .and_then(|s| s.get(rest))
            .map(|n| json!(n));
    }
    if let Some(rest) = value.strip_prefix("s-") {
        return doc
            .scales
            .get("space")
            .and_then(|s| s.get(rest))
            .map(|n| json!(n));
    }
    None
}

/// Built-in dark theme ASL fragment matching `docs/design-system/02-style/`.
pub fn design_system_dark_asl() -> &'static str {
    r#"
token surface.canvas = #0B0C13
token surface.sunken = #05050A
token surface.card = #1C1E2B
token surface.raised = #292B3C
token text.primary = #F7F8FC
token text.secondary = #A2A6BB
token text.tertiary = #82869C
token text.on-accent = #12131C
token accent.default = #9C7CF2
token border.default = #3B3E52
token border.focus = #9C7CF2
scale radius: sm=6 md=10 lg=16 xl=24
scale space: xs=4 sm=8 md=12 lg=16 xl=24 xxl=32 xxxl=48
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_design_system_fragment() {
        let doc = parse_asl(design_system_dark_asl());
        assert_eq!(
            doc.tokens.get("accent.default").map(String::as_str),
            Some("#9C7CF2")
        );
        assert_eq!(
            doc.scales.get("radius").and_then(|s| s.get("md")),
            Some(&10)
        );
        assert_eq!(
            resolve_ref(&doc, "token:text.primary"),
            Some(json!("#F7F8FC"))
        );
        assert_eq!(resolve_ref(&doc, "r-md"), Some(json!(10)));
    }
}
