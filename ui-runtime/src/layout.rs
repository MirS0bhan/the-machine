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
    /// `label` was shortened to fit `width`; `full_label` keeps the original so
    /// assistive tech and tooltips still read the whole string.
    pub truncated: bool,
    pub full_label: String,
    /// Motion curve requested by the node (`motion=gentle`), empty for default.
    pub motion: String,
}

/// Nominal glyph advance for a font scale, used to fit labels to a box.
///
/// Approximate on purpose: the boot renderer has no shaping feedback loop, so
/// layout budgets width from the same table the renderer picks `font_px` from.
pub fn glyph_advance(font_scale: u32) -> u32 {
    match font_scale {
        4 => 11,
        3 => 7,
        2 => 7,
        1 => 6,
        _ => 7,
    }
}

/// Characters that fit in `width` at `font_scale`, leaving room for padding.
fn fitting_chars(width: u32, font_scale: u32, padding: u32) -> usize {
    let usable = width.saturating_sub(padding);
    (usable / glyph_advance(font_scale).max(1)) as usize
}

/// Shorten `label` to fit, appending an ellipsis. Multi-line labels (the chat
/// log) are clipped per line so the newest turn stays readable.
pub fn fit_label(label: &str, width: u32, font_scale: u32, padding: u32) -> (String, bool) {
    let budget = fitting_chars(width, font_scale, padding);
    if budget == 0 {
        return (String::new(), !label.is_empty());
    }
    if !label.contains('\n') {
        if label.chars().count() <= budget {
            return (label.to_string(), false);
        }
        let keep = budget.saturating_sub(1).max(1);
        let mut out: String = label.chars().take(keep).collect();
        out.push('…');
        return (out, true);
    }
    let mut truncated = false;
    let lines: Vec<String> = label
        .lines()
        .map(|line| {
            if line.chars().count() <= budget {
                line.to_string()
            } else {
                truncated = true;
                let keep = budget.saturating_sub(1).max(1);
                let mut out: String = line.chars().take(keep).collect();
                out.push('…');
                out
            }
        })
        .collect();
    (lines.join("\n"), truncated)
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
                tokens::cur::surface_canvas(),
                if is_caption {
                    tokens::cur::text_secondary()
                } else {
                    tokens::cur::text_primary()
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
                tokens::cur::text_tertiary()
            } else {
                tokens::cur::text_primary()
            };
            (
                tokens::cur::surface_sunken(),
                fg,
                Some(tokens::cur::border_default()),
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
                    tokens::cur::accent_pressed()
                } else if hovered {
                    tokens::cur::accent_hover()
                } else {
                    tokens::cur::accent_default()
                };
                (bg, tokens::cur::text_on_accent())
            } else {
                let bg = if pressed {
                    tokens::cur::surface_card()
                } else {
                    tokens::cur::surface_raised()
                };
                (bg, tokens::cur::text_primary())
            };
            (
                bg,
                fg,
                if pressed {
                    Some(tokens::cur::border_focus())
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
            tokens::cur::surface_card(),
            tokens::cur::text_primary(),
            Some(tokens::cur::border_default()),
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

    // Long localized copy (or a pseudo-locale) must not run past the box.
    let padding = match kind.as_str() {
        "button" => 32,
        "field" | "input" => 24,
        _ => 8,
    };
    let (label, truncated) = fit_label(&label, w, font_scale, padding);
    let full_label = if truncated {
        // Recompute the untruncated string for AT / tooltips.
        match kind.as_str() {
            "text" => text.clone(),
            "field" | "input" => {
                if placeholder_active {
                    placeholder.clone()
                } else {
                    text.clone()
                }
            }
            _ => label_prop.clone(),
        }
    } else {
        label.clone()
    };

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
        truncated,
        full_label,
        motion: crate::motion::requested_curve(&props).unwrap_or_default(),
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

    #[test]
    fn fit_label_ellipsises_only_when_needed() {
        let (short, cut) = fit_label("Send", 200, 3, 32);
        assert_eq!(short, "Send");
        assert!(!cut);
        let long = "Send this extremely long localized button label somewhere";
        let (fitted, cut) = fit_label(long, 120, 3, 32);
        assert!(cut);
        assert!(fitted.ends_with('…'));
        assert!(fitted.chars().count() < long.chars().count());
    }

    #[test]
    fn fit_label_clips_each_line_of_a_multiline_log() {
        let log = "You: a short line\nAssistant: a very much longer line that will not fit at all";
        let (fitted, cut) = fit_label(log, 160, 2, 8);
        assert!(cut);
        assert_eq!(fitted.lines().count(), 2);
        assert!(fitted.lines().next().unwrap().starts_with("You: a short"));
        assert!(fitted.lines().nth(1).unwrap().ends_with('…'));
    }

    #[test]
    fn zero_width_box_drops_the_label_but_reports_truncation() {
        let (fitted, cut) = fit_label("something", 4, 3, 32);
        assert!(fitted.is_empty());
        assert!(cut);
    }

    #[test]
    fn overlong_locale_copy_is_fitted_and_full_text_kept() {
        let root = json!({
            "id": "ui.root",
            "type": "stack",
            "props": { "dir": "v" },
            "children": [{
                "id": "ui.chat_send",
                "type": "button",
                "props": {
                    "label": "[⟦Send this pseudo-localized label that overflows ⟧⟧⟧]",
                    "variant": "primary"
                }
            }]
        });
        let nodes = layout_tree(&root, 1280, 720);
        let button = &nodes[0];
        assert!(
            button.truncated,
            "label {:?} should be fitted",
            button.label
        );
        assert!(button.label.ends_with('…'));
        assert!(button.full_label.contains("pseudo-localized"));
    }

    #[test]
    fn rtl_prop_mirrors_horizontal_children() {
        let root = json!({
            "id": "ui.root",
            "type": "stack",
            "props": { "dir": "h", "gap": "sm", "rtl": true },
            "children": [
                { "id": "a", "type": "button", "props": { "label": "A" } },
                { "id": "b", "type": "button", "props": { "label": "B" } },
            ]
        });
        let nodes = layout_tree(&root, 1280, 720);
        let a = nodes.iter().find(|n| n.id == "a").unwrap();
        let b = nodes.iter().find(|n| n.id == "b").unwrap();
        assert!(a.x > b.x, "first child should sit right of second in RTL");
    }
}
