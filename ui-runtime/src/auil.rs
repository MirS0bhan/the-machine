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
        .filter(|(_, l)| !l.trim().is_empty())
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

    let mut rest = head;
    if let Some(sp) = rest.find(char::is_whitespace) {
        let (t, p) = rest.split_at(sp);
        parse_tag_token(t, &mut tag, &mut id)?;
        parse_props(p.trim(), &mut props);
    } else {
        parse_tag_token(rest, &mut tag, &mut id)?;
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

fn parse_tag_token(token: &str, tag: &mut String, id: &mut Option<String>) -> Result<(), String> {
    let mut base = token;
    if let Some(hash) = token.find('#') {
        base = &token[..hash];
        *id = Some(token[hash + 1..].to_string());
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

fn split_text_content(line: &str) -> (&str, Option<String>) {
    if let Some(start) = line.find('"') {
        if let Some(end) = line.rfind('"') {
            if end > start {
                let text = line[start + 1..end].to_string();
                return (&line[..start], Some(text));
            }
        }
    }
    (line, None)
}

/// Convert an AUIL tree into `ui.patch` insert operations.
pub fn auil_to_patch_ops(root: &AuilNode, parent_id: &str) -> Vec<Value> {
    let mut ops = Vec::new();
    flatten_node(root, parent_id, &mut ops);
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
    fn parses_simple_stack() {
        let src = r#"stack#ui.root dir=v
  text#title(role=title) "Hello"
  button#ok label=OK on:press=mcp:ui.status"#;
        let tree = parse_auil(src).unwrap();
        assert_eq!(tree.id.as_deref(), Some("ui.root"));
        assert_eq!(tree.children.len(), 2);
        let ops = auil_to_patch_ops(&tree, "ui.root");
        assert!(ops.len() >= 3);
        assert!(ops[0].get("node").unwrap().get("id").is_some());
    }
}
