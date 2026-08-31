//! Minimal stack layout for AUIL trees (boot path).
//!
//! Implements vertical/horizontal stack with gap, pad, and align=center —
//! enough for SessionGreeting without a full flex engine.

use serde_json::{json, Value};

use crate::tokens::{self, space, type_size, Rgb};

#[derive(Clone, Debug)]
pub struct LaidOutNode {
    pub id: String,
    pub kind: String,
    pub label: String,
    #[allow(dead_code)]
    pub placeholder: String,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub bg: Rgb,
    pub fg: Rgb,
    pub border: Option<Rgb>,
    pub radius: u32,
    pub font_scale: u32,
    pub variant: String,
    #[allow(dead_code)]
    pub role: String,
    pub pressed: bool,
    pub checked: bool,
    pub value: f64,
    pub value_min: f64,
    pub value_max: f64,
    pub scroll_y: i32,
    pub items: Vec<String>,
    pub caret: i64,
    pub placeholder_active: bool,
    pub src: String,
}

/// Viewport used when compositor size is unknown (memory/DRM defaults).
pub fn default_viewport() -> (u32, u32) {
    let w = std::env::var("THE_MACHINE_FB_WIDTH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1280);
    let h = std::env::var("THE_MACHINE_FB_HEIGHT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(720);
    (w, h)
}

pub fn layout_tree(root: &Value, viewport_w: u32, viewport_h: u32) -> Vec<LaidOutNode> {
    let mut out = Vec::new();
    let pad = space::XXXL as i32;
    let content_w = viewport_w.saturating_sub((pad * 2) as u32).max(320);
    let origin_x = pad;
    let origin_y = pad;
    layout_node(
        root,
        origin_x,
        origin_y,
        content_w,
        viewport_h.saturating_sub((pad * 2) as u32),
        &mut out,
    );
    out
}

fn layout_node(
    node: &Value,
    x: i32,
    y: i32,
    avail_w: u32,
    avail_h: u32,
    out: &mut Vec<LaidOutNode>,
) {
    let kind = node.get("type").and_then(|v| v.as_str()).unwrap_or("stack");
    let _id = node
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("ui.unknown")
        .to_string();
    let props = node.get("props").cloned().unwrap_or(json!({}));

    if kind == "grid" {
        layout_grid(node, &props, x, y, avail_w, avail_h, out);
        return;
    }

    if matches!(kind, "stack" | "container") {
        let dir = props.get("dir").and_then(|v| v.as_str()).unwrap_or("v");
        let rtl = props.get("rtl").and_then(|v| v.as_bool()).unwrap_or(false)
            || dir == "rtl"
            || props
                .get("writing_mode")
                .and_then(|v| v.as_str())
                .is_some_and(|w| w == "rtl")
            || crate::i18n::active_rtl();
        let dir = if dir == "rtl" { "h" } else { dir };
        let gap = gap_px(&props);
        let align_center = props
            .get("align")
            .and_then(|v| v.as_str())
            .map(|a| a == "center")
            .unwrap_or(false);
        let children = node
            .get("children")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Measure children heights for vertical centering of the greeting block.
        let mut measured: Vec<(Value, u32, u32)> = Vec::new();
        for child in &children {
            let (w, h) = measure_leaf(child, avail_w);
            // Skip collapsed placeholders (e.g. empty chat_log caption pre-fill).
            if w == 0 && h == 0 {
                let child_kind = child.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if !matches!(child_kind, "stack" | "container" | "grid") {
                    continue;
                }
            }
            measured.push((child.clone(), w, h));
        }
        if rtl && dir == "h" {
            measured.reverse();
        }
        let total_h: u32 = if dir == "v" {
            measured.iter().map(|(_, _, h)| *h).sum::<u32>()
                + gap * measured.len().saturating_sub(1) as u32
        } else {
            measured.iter().map(|(_, _, h)| *h).max().unwrap_or(0)
        };
        let total_w: u32 = if dir == "h" {
            measured.iter().map(|(_, w, _)| *w).sum::<u32>()
                + gap * measured.len().saturating_sub(1) as u32
        } else {
            measured.iter().map(|(_, w, _)| *w).max().unwrap_or(avail_w)
        };

        let mut cursor_x = x;
        let mut cursor_y = y;
        if align_center && dir == "v" && avail_h > total_h {
            cursor_y = y + ((avail_h - total_h) / 2) as i32;
        }
        if align_center && dir == "h" && avail_w > total_w {
            cursor_x = x + ((avail_w - total_w) / 2) as i32;
        }

        for (child, cw, ch) in measured {
            let child_kind = child
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("stack");
            if matches!(child_kind, "stack" | "container" | "grid") {
                let cx = if align_center && dir == "v" {
                    x + ((avail_w.saturating_sub(cw)) / 2) as i32
                } else {
                    cursor_x
                };
                layout_node(&child, cx, cursor_y, cw.min(avail_w), ch, out);
            } else {
                let cx = if align_center && dir == "v" {
                    x + ((avail_w.saturating_sub(cw)) / 2) as i32
                } else {
                    cursor_x
                };
                out.push(style_leaf(&child, cx, cursor_y, cw, ch));
            }
            if dir == "v" {
                cursor_y += (ch + gap) as i32;
            } else {
                cursor_x += (cw + gap) as i32;
            }
        }
        return;
    }

    let (w, h) = measure_leaf(node, avail_w);
    out.push(style_leaf(node, x, y, w.min(avail_w), h));
}

fn layout_grid(
    node: &Value,
    props: &Value,
    x: i32,
    y: i32,
    avail_w: u32,
    _avail_h: u32,
    out: &mut Vec<LaidOutNode>,
) {
    let cols = props
        .get("cols")
        .or_else(|| props.get("columns"))
        .and_then(|v| v.as_u64())
        .unwrap_or(2) as u32;
    let gap = gap_px(props);
    let rtl = props.get("rtl").and_then(|v| v.as_bool()).unwrap_or(false)
        || props
            .get("dir")
            .and_then(|v| v.as_str())
            .is_some_and(|d| d == "rtl")
        || crate::i18n::active_rtl();
    let children = node
        .get("children")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut sizes = Vec::new();
    let mut child_props = Vec::new();
    let mut kept = Vec::new();
    for child in children {
        let (w, h) = measure_leaf(&child, avail_w / cols.max(1));
        if w == 0 && h == 0 {
            continue;
        }
        child_props.push(child.get("props").cloned().unwrap_or(json!({})));
        sizes.push((w, h));
        kept.push(child);
    }
    let plan = crate::grid::plan(cols, gap, avail_w, &sizes, &child_props, rtl);
    for cell in &plan.cells {
        let child = &kept[cell.child_index];
        let (cw, ch) = sizes[cell.child_index];
        let (cx, cy) = plan.origin_of(cell, x, y);
        let (pw, ph) = plan.size_of(cell, cw, ch);
        let child_kind = child
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("stack");
        if matches!(child_kind, "stack" | "container" | "grid") {
            layout_node(child, cx, cy, pw, ph, out);
        } else {
            out.push(style_leaf(child, cx, cy, cw.min(pw), ch.min(ph.max(1))));
        }
    }
}

fn gap_px(props: &Value) -> u32 {
    match props.get("gap").and_then(|v| v.as_str()).unwrap_or("lg") {
        "xs" | "s-xs" => space::XS,
        "sm" | "s-sm" | "s" => space::SM,
        "md" | "s-md" | "m" => space::MD,
        "xl" | "s-xl" => space::XL,
        "xxl" | "s-xxl" => space::XXL,
        _ => space::LG,
    }
}

fn measure_leaf(node: &Value, avail_w: u32) -> (u32, u32) {
    let kind = node.get("type").and_then(|v| v.as_str()).unwrap_or("text");
    let id = node.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let props = node.get("props").cloned().unwrap_or(json!({}));
    let role = props.get("role").and_then(|v| v.as_str()).unwrap_or("");

    match kind {
        "text" => {
            let text = props.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if (role == "caption" || id == "ui.chat_log") && text.is_empty() {
                return (0, 0);
            }
            let h = if role == "title" || id == "ui.greeting" {
                48
            } else if id == "ui.chat_log" {
                let lines = text.lines().filter(|l| !l.is_empty()).count().max(1) as u32;
                (24 * lines.min(14)).max(48)
            } else if role == "caption" {
                28
            } else {
                36
            };
            let w = if role == "title" {
                (avail_w * 3 / 4).clamp(280, 720)
            } else if id == "ui.chat_log" {
                (avail_w * 4 / 5).clamp(320, 800)
            } else {
                (avail_w * 2 / 3).clamp(240, 640)
            };
            (w, h)
        }
        "field" | "input" => {
            let w = (avail_w * 2 / 3).clamp(320, 560);
            (w, space::MIN_TARGET.max(52))
        }
        "button" => {
            let label = props
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("Send");
            let w = (label.len() as u32 * 12 + 48).clamp(space::MIN_TARGET, 200);
            (w, space::MIN_TARGET.max(48))
        }
        "icon" => {
            let (w, h) = crate::widgets::measure("icon", &props, avail_w);
            (w, h)
        }
        "toggle" | "slider" | "list" | "dialog" | "media" | "chart" => {
            crate::widgets::measure(kind, &props, avail_w)
        }
        _ => (240, 48),
    }
}

fn style_leaf(node: &Value, x: i32, y: i32, w: u32, h: u32) -> LaidOutNode {
    let kind = node
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("widget")
        .to_string();
    let id = node
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("ui.unknown")
        .to_string();
    let props = node.get("props").cloned().unwrap_or(json!({}));
    let role = props
        .get("role")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let text = crate::i18n::resolve_label(props.get("text").and_then(|v| v.as_str()).unwrap_or(""));
    let label_prop =
        crate::i18n::resolve_label(props.get("label").and_then(|v| v.as_str()).unwrap_or(""));
    let placeholder = crate::i18n::resolve_label(
        props
            .get("placeholder")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let variant = props
        .get("variant")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let (bg, fg, border, radius, font_scale, label, variant) = match kind.as_str() {
        "text" => {
            let is_title = role == "title" || id == "ui.greeting";
            let is_caption = role == "caption" || id == "ui.chat_log";
            let label = if text.is_empty() {
                // Keep caption empty rather than echoing the widget id on canvas.
                String::new()
            } else {
                text.to_string()
            };
            (
                tokens::dark::SURFACE_CANVAS,
                if is_caption {
                    tokens::dark::TEXT_SECONDARY
                } else {
                    tokens::dark::TEXT_PRIMARY
                },
                None,
                0,
                if is_title {
                    4
                } else if is_caption {
                    2
                } else {
                    3
                },
                label,
                String::new(),
            )
        }
        "field" | "input" => {
            let shown = if text.is_empty() {
                if placeholder.is_empty() {
                    String::new()
                } else {
                    placeholder.clone()
                }
            } else {
                text.to_string()
            };
            let fg = if text.is_empty() {
                tokens::dark::TEXT_TERTIARY
            } else {
                tokens::dark::TEXT_PRIMARY
            };
            (
                tokens::dark::SURFACE_SUNKEN,
                fg,
                Some(tokens::dark::BORDER_DEFAULT),
                tokens::radius::MD,
                3,
                shown,
                "field".into(),
            )
        }
        "button" => {
            let v = if variant.is_empty() {
                "primary".to_string()
            } else {
                variant.clone()
            };
            let pressed = props
                .get("pressed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let hovered = props
                .get("hovered")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let (bg, fg) = if v == "primary" {
                let bg = if pressed {
                    tokens::dark::ACCENT_PRESSED
                } else if hovered {
                    tokens::dark::ACCENT_HOVER
                } else {
                    tokens::dark::ACCENT_DEFAULT
                };
                (bg, tokens::dark::TEXT_ON_ACCENT)
            } else {
                let bg = if pressed {
                    tokens::dark::SURFACE_CARD
                } else {
                    tokens::dark::SURFACE_RAISED
                };
                (bg, tokens::dark::TEXT_PRIMARY)
            };
            (
                bg,
                fg,
                if pressed {
                    Some(tokens::dark::BORDER_FOCUS)
                } else {
                    None
                },
                tokens::radius::MD,
                3,
                if label_prop.is_empty() {
                    "Send".into()
                } else {
                    label_prop.to_string()
                },
                v,
            )
        }
        "toggle" | "slider" | "dialog" | "list" | "icon" | "media" | "chart" => {
            let chrome = crate::widgets::style(&kind, &id, &props);
            (
                chrome.bg,
                chrome.fg,
                chrome.border,
                chrome.radius,
                chrome.font_scale,
                chrome.label,
                chrome.variant,
            )
        }
        _ => (
            tokens::dark::SURFACE_CARD,
            tokens::dark::TEXT_PRIMARY,
            Some(tokens::dark::BORDER_DEFAULT),
            tokens::radius::MD,
            2,
            if !text.is_empty() {
                text.to_string()
            } else if !label_prop.is_empty() {
                label_prop.to_string()
            } else {
                id.clone()
            },
            variant,
        ),
    };

    // Silence unused — type scale documented alongside font_scale mapping.
    let _ = (type_size::BODY, type_size::TITLE_2, type_size::CAPTION);

    let pressed = props
        .get("pressed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let checked = props
        .get("checked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let value_min = props.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let value_max = props.get("max").and_then(|v| v.as_f64()).unwrap_or(100.0);
    let value = props
        .get("value")
        .and_then(|v| v.as_f64())
        .unwrap_or(value_min);
    let scroll_y = props.get("scroll_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let items: Vec<String> = props
        .get("items")
        .or_else(|| props.get("data"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| v.as_f64().map(|n| n.to_string()))
                        .or_else(|| v.as_i64().map(|n| n.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();
    let caret = props.get("caret").and_then(|v| v.as_i64()).unwrap_or(
        if matches!(kind.as_str(), "field" | "input") {
            text.len() as i64
        } else {
            -1
        },
    );
    let placeholder_active =
        matches!(kind.as_str(), "field" | "input") && text.is_empty() && !placeholder.is_empty();
    let src = props
        .get("src")
        .or_else(|| props.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    LaidOutNode {
        id,
        kind,
        label,
        placeholder,
        x,
        y,
        width: w,
        height: h,
        bg,
        fg,
        border,
        radius,
        font_scale,
        variant,
        role,
        pressed,
        checked,
        value,
        value_min,
        value_max,
        scroll_y,
        items,
        caret,
        placeholder_active,
        src,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lays_out_greeting_stack_centered() {
        let root = json!({
            "id": "ui.root",
            "type": "stack",
            "props": { "dir": "v", "gap": "lg", "align": "center" },
            "children": [
                { "id": "ui.greeting", "type": "text", "props": { "role": "title", "text": "Welcome back" } },
                { "id": "ui.chat_input", "type": "field", "props": { "placeholder": "Ask or say what you need" } },
                { "id": "ui.chat_send", "type": "button", "props": { "label": "Send", "variant": "primary" } },
            ]
        });
        let nodes = layout_tree(&root, 1280, 720);
        assert_eq!(nodes.len(), 3);
        assert_eq!(nodes[0].id, "ui.greeting");
        assert_eq!(nodes[1].kind, "field");
        assert_eq!(nodes[2].variant, "primary");
        // Greeting should sit roughly in the middle vertically.
        assert!(nodes[0].y > 100, "y={}", nodes[0].y);
        assert_eq!(nodes[2].bg, tokens::dark::ACCENT_DEFAULT);
    }
}
