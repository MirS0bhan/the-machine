//! AUIL parser — Rust port of `ui-engine/auil_parser.py` for the boot path.

use std::collections::HashMap;

use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct AuilNode {
    pub tag: String,
    pub id: Option<String>,
    pub props: HashMap<String, String>,
    pub text: Option<String>,
    pub children: Vec<AuilNode>,
}

/// Parse AUIL source into a tree (indentation-based, 2-space levels).
pub fn parse_auil(source: &str) -> Result<AuilNode, String> {
    let lines: Vec<(usize, &str)> = source
        .lines()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .map(|(i, l)| (i + 1, l))
        .collect();
    if lines.is_empty() {
        return Ok(AuilNode {
            tag: "stack".into(),
            id: Some("ui.root".into()),
            props: HashMap::new(),
            text: None,
            children: vec![],
        });
    }
    let (root, _) = parse_block(&lines, 0, lines.len(), 0)?;
    Ok(root)
}

fn parse_block(
    lines: &[(usize, &str)],
    start: usize,
    end: usize,
    base_indent: usize,
) -> Result<(AuilNode, usize), String> {
    if start >= end {
        return Err("empty AUIL block".into());
    }
    let (line_no, line) = lines[start];
    let node = parse_line(line).map_err(|e| format!("line {line_no}: {e}"))?;
    let mut children = Vec::new();
    let mut i = start + 1;
    let mut child_indent = None;
    while i < end {
        let (_, content) = lines[i];
        let indent = content.len() - content.trim_start().len();
        if indent <= base_indent {
            break;
        }
        if child_indent.is_none() {
            child_indent = Some(indent);
        }
        if indent == child_indent.unwrap() {
            let (child, next) = parse_block(lines, i, end, indent)?;
            children.push(child);
            i = next;
        } else if indent > child_indent.unwrap() {
            i += 1;
        } else {
            break;
        }
    }
    let mut node = node;
    node.children = children;
    Ok((node, i))
}

fn parse_line(line: &str) -> Result<AuilNode, String> {
    let line = line.trim();
    let (head, text) = split_text_content(line);
    let mut tag = String::new();
    let mut id = None;
    let mut props = HashMap::new();

    // Pull the parenthesised prop group out first: it may contain spaces and
    // quoted values, so it cannot survive naive whitespace splitting.
    let (head, paren) = split_paren_group(head);
    if let Some(group) = paren {
        parse_props(&group, &mut props);
    }

    let rest = head.trim();
    if let Some(sp) = rest.find(char::is_whitespace) {
        let (t, p) = rest.split_at(sp);
        parse_tag_token(t, &mut tag, &mut id, &mut props)?;
        parse_props(p.trim(), &mut props);
    } else {
        parse_tag_token(rest, &mut tag, &mut id, &mut props)?;
    }

    if tag.is_empty() {
        return Err(format!("invalid AUIL line: {line}"));
    }
    Ok(AuilNode {
        tag,
        id,
        props,
        text,
        children: vec![],
    })
}

/// Split `tag#id(a=1 b="two words") rest` into (`tag#id rest`, `(a=1 b="two words")`).
///
/// Quotes are honoured so a `)` inside a quoted value does not close the group.
fn split_paren_group(head: &str) -> (String, Option<String>) {
    let Some(open) = head.find('(') else {
        return (head.to_string(), None);
    };
    let bytes: Vec<char> = head.chars().collect();
    let open_idx = head[..open].chars().count();
    let mut in_quotes = false;
    let mut depth = 0usize;
    for (i, ch) in bytes.iter().enumerate().skip(open_idx) {
        match ch {
            '"' => in_quotes = !in_quotes,
            '(' if !in_quotes => depth += 1,
            ')' if !in_quotes => {
                depth -= 1;
                if depth == 0 {
                    let group: String = bytes[open_idx..=i].iter().collect();
                    let before: String = bytes[..open_idx].iter().collect();
                    let after: String = bytes[i + 1..].iter().collect();
                    return (format!("{before} {after}"), Some(group));
                }
            }
            _ => {}
        }
    }
    (head.to_string(), None)
}

fn parse_tag_token(
    token: &str,
    tag: &mut String,
    id: &mut Option<String>,
    props: &mut HashMap<String, String>,
) -> Result<(), String> {
    let mut base = token;
    if let Some(hash) = token.find('#') {
        base = &token[..hash];
        let id_part = &token[hash + 1..];
        if let Some(paren) = id_part.find('(') {
            *id = Some(id_part[..paren].to_string());
            if let Some(end) = id_part.rfind(')') {
                parse_props(&id_part[paren..=end], props);
            }
        } else {
            *id = Some(id_part.to_string());
        }
    }
    if let Some(paren) = base.find('(') {
        *tag = base[..paren].to_string();
    } else if let Some(dot) = base.find('.') {
        *tag = base[..dot].to_string();
    } else {
        *tag = base.to_string();
    }
    if tag.is_empty() {
        return Err(format!("missing tag in {token}"));
    }
    Ok(())
}

fn parse_props(s: &str, props: &mut HashMap<String, String>) {
    if s.is_empty() {
        return;
    }
    if let Some(inner) = s.strip_prefix('(').and_then(|x| x.strip_suffix(')')) {
        for token in tokenize_props(inner) {
            if let Some((k, v)) = token.split_once('=') {
                props.insert(k.to_string(), strip_quotes(v));
            }
        }
        return;
    }
    for token in tokenize_props(s) {
        if let Some((k, v)) = token.split_once('=') {
            props.insert(k.to_string(), strip_quotes(v));
        } else if token.contains(':') {
            props.insert(token.to_string(), "true".into());
        }
    }
}

fn tokenize_props(s: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    for ch in s.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            cur.push(ch);
        } else if ch == ' ' && !in_quotes {
            if !cur.is_empty() {
                tokens.push(cur.clone());
                cur.clear();
            }
        } else {
            cur.push(ch);
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

/// Text content is the quoted run at the **end** of the line, so quoted prop
/// values earlier on the line are left for the prop parser.
fn split_text_content(line: &str) -> (&str, Option<String>) {
    let trimmed = line.trim_end();
    if trimmed.len() < 2 || !trimmed.ends_with('"') {
        return (line, None);
    }
    let close = trimmed.len() - 1;
    if let Some(open) = trimmed[..close].rfind('"') {
        let text = trimmed[open + 1..close].to_string();
        return (&line[..open], Some(text));
    }
    (line, None)
}

/// Convert an AUIL tree into `ui.patch` insert operations.
pub fn auil_to_patch_ops(root: &AuilNode, parent_id: &str) -> Vec<Value> {
    let mut ops = Vec::new();
    let root_id = root
        .id
        .as_deref()
        .unwrap_or(parent_id);
    if root_id == parent_id {
        // Boot file root matches the live tree root — insert children only.
        for child in &root.children {
            flatten_node(child, parent_id, &mut ops);
        }
    } else {
        flatten_node(root, parent_id, &mut ops);
    }
    ops
}

fn flatten_node(node: &AuilNode, anchor: &str, ops: &mut Vec<Value>) {
    let id = node
        .id
        .clone()
        .unwrap_or_else(|| format!("ui.{}", node.tag));
    let mut props: HashMap<String, Value> = node
        .props
        .iter()
        .filter(|(k, _)| !k.starts_with("on:"))
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    if let Some(text) = &node.text {
        props.insert("text".into(), Value::String(text.clone()));
    }
    let bindings: Vec<Value> = node
        .props
        .iter()
        .filter(|(k, _)| k.starts_with("on:"))
        .map(|(k, v)| {
            let event = k.strip_prefix("on:").unwrap_or("press");
            let (kind, target) = if let Some(m) = v.strip_prefix("mcp:") {
                ("mcp", m)
            } else if let Some(p) = v.strip_prefix("state:") {
                ("state", p)
            } else {
                ("mcp", v.as_str())
            };
            json!({ "type": kind, "target": target, "event": event })
        })
        .collect();

    let mut node_json = json!({
        "id": id,
        "type": node.tag,
        "props": props,
    });
    if !bindings.is_empty() {
        node_json
            .as_object_mut()
            .unwrap()
            .insert("bindings".into(), Value::Array(bindings));
    }

    ops.push(json!({
        "op": "insert",
        "anchor": anchor,
        "position": "child",
        "node": node_json,
    }));

    for child in &node.children {
        flatten_node(child, &id, ops);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skips_hash_comment_lines() {
        let src = r#"# Boot layout comment
stack#ui.root
  text#ui.greeting "Hi"
"#;
        let tree = parse_auil(src).unwrap();
        assert_eq!(tree.id.as_deref(), Some("ui.root"));
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].id.as_deref(), Some("ui.greeting"));
    }

    #[test]
    fn boot_auil_does_not_insert_root_under_itself() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build/boot.auil");
        let src = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            r#"stack#ui.root dir=v gap=md
  stack#ui.chrome dir=h
    text#ui.status_line(role=caption) "The Machine"
    text#ui.activity(role=caption) ""
  stack#ui.session dir=v
    text#ui.greeting "Hi"
    text#ui.chat_log(role=caption) ""
    field#ui.chat_input ""
    button#ui.chat_send label=Send on:press=mcp:agent.chat.send
  stack#ui.workspace dir=v
    text#ui.workspace_hint(role=caption) ""
"#
            .into()
        });
        let tree = parse_auil(&src).unwrap();
        let ops = auil_to_patch_ops(&tree, "ui.root");
        assert!(!ops.is_empty());
        for op in &ops {
            let id = op
                .get("node")
                .and_then(|n| n.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            assert_ne!(id, "ui.root", "must not re-insert ui.root under itself");
        }
        let ids: Vec<_> = ops
            .iter()
            .filter_map(|op| op.get("node").and_then(|n| n.get("id")).and_then(|v| v.as_str()))
            .collect();
        for expected in [
            "ui.chrome",
            "ui.status_line",
            "ui.activity",
            "ui.session",
            "ui.greeting",
            "ui.chat_log",
            "ui.chat_input",
            "ui.chat_send",
            "ui.workspace",
            "ui.workspace_hint",
        ] {
            assert!(
                ids.contains(&expected),
                "boot.auil missing {expected} in patch ops: {ids:?}"
            );
        }
    }

    #[test]
    fn parses_boot_widget_ids_without_inline_props() {
        let src = r#"stack#ui.root
  text#ui.greeting(role=title) "Hi"
  button#ui.chat_send label=Send on:press=mcp:agent.chat.send"#;
        let tree = parse_auil(src).unwrap();
        let ops = auil_to_patch_ops(&tree, "ui.root");
        let ids: Vec<_> = ops
            .iter()
            .filter_map(|op| op.get("node").and_then(|n| n.get("id")).and_then(|v| v.as_str()))
            .collect();
        assert!(ids.contains(&"ui.greeting"));
        assert!(ids.contains(&"ui.chat_send"));
        assert!(!ids.iter().any(|id| id.contains('(')));
    }

    #[test]
    fn parses_paren_props_containing_spaces_and_quotes() {
        let src = r#"stack#ui.root
  field#ui.chat_input(input-mode=hybrid placeholder="Ask or say what you need" aria-label="Chat") ""
  list#ui.suggestions(label="Suggestions" height=96) on:activate=mcp:agent.chat.send"#;
        let tree = parse_auil(src).unwrap();
        let field = &tree.children[0];
        assert_eq!(field.id.as_deref(), Some("ui.chat_input"));
        assert_eq!(field.props.get("input-mode").map(String::as_str), Some("hybrid"));
        assert_eq!(
            field.props.get("placeholder").map(String::as_str),
            Some("Ask or say what you need")
        );
        assert_eq!(field.props.get("aria-label").map(String::as_str), Some("Chat"));
        assert_eq!(field.text.as_deref(), Some(""));

        let list = &tree.children[1];
        assert_eq!(list.props.get("label").map(String::as_str), Some("Suggestions"));
        assert_eq!(list.props.get("height").map(String::as_str), Some("96"));
        assert_eq!(
            list.props.get("on:activate").map(String::as_str),
            Some("mcp:agent.chat.send")
        );
    }

    #[test]
    fn boot_auil_carries_i18n_chrome_and_suggestions() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../build/boot.auil");
        let Ok(src) = std::fs::read_to_string(&path) else {
            return;
        };
        let tree = parse_auil(&src).unwrap();
        let ops = auil_to_patch_ops(&tree, "ui.root");
        let find = |id: &str| {
            ops.iter()
                .find(|op| op["node"]["id"] == id)
                .cloned()
                .unwrap_or_else(|| panic!("missing {id}"))
        };
        assert_eq!(find("ui.chat_input")["node"]["props"]["placeholder"], "i18n:chat.placeholder");
        assert_eq!(find("ui.chat_send")["node"]["props"]["label"], "i18n:chat.send");
        assert_eq!(find("ui.greeting")["node"]["props"]["text"], "i18n:app.welcome");
        assert_eq!(find("ui.suggestions")["node"]["type"], "list");
        assert_eq!(find("ui.activity")["node"]["props"]["live"], "polite");
        assert_eq!(
            find("ui.chat_send")["node"]["bindings"][0]["target"],
            "agent.chat.send"
        );
    }

    #[test]
    fn parses_simple_stack() {
        let src = r#"stack#ui.root dir=v
  text#title(role=title) "Hello"
  button#ok label=OK on:press=mcp:ui.status"#;
        let tree = parse_auil(src).unwrap();
        assert_eq!(tree.id.as_deref(), Some("ui.root"));
        assert_eq!(tree.children.len(), 2);
        let ops = auil_to_patch_ops(&tree, "ui.root");
        assert_eq!(ops.len(), 2);
        assert!(ops.iter().all(|op| op.get("node").and_then(|n| n.get("id")).is_some()));
    }
}
