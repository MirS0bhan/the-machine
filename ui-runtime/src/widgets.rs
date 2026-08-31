//! Per-kind measure/style helpers beyond the SessionGreeting leaf set.

use serde_json::{json, Value};

use crate::tokens::{self, space, Rgb};

#[derive(Clone, Debug)]
pub struct WidgetChrome {
    pub bg: Rgb,
    pub fg: Rgb,
    pub border: Option<Rgb>,
    pub radius: u32,
    pub font_scale: u32,
    pub label: String,
    pub variant: String,
}

pub fn measure(kind: &str, props: &Value, avail_w: u32) -> (u32, u32) {
    match kind {
        "toggle" => (space::MIN_TARGET.max(52), space::MIN_TARGET),
        "slider" => ((avail_w * 2 / 3).clamp(160, 480), 28),
        "icon" => {
            let size = match props.get("size").and_then(|v| v.as_str()).unwrap_or("md") {
                "sm" | "icon.sm" => 16,
                "lg" | "icon.lg" => 32,
                "xl" | "icon.xl" => 48,
                _ => 24,
            };
            (size, size)
        }
        "list" => (avail_w.saturating_sub(space::LG), 160),
        "dialog" => ((avail_w * 3 / 4).clamp(280, 560), 240),
        "media" => ((avail_w * 2 / 3).clamp(240, 640), 180),
        "chart" => ((avail_w * 2 / 3).clamp(240, 640), 160),
        _ => (240, 48),
    }
}

pub fn style(kind: &str, id: &str, props: &Value) -> WidgetChrome {
    let label = props
        .get("label")
        .or_else(|| props.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or(id)
        .to_string();
    let variant = props
        .get("variant")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    match kind {
        "toggle" => {
            let on = props
                .get("checked")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            WidgetChrome {
                bg: if on {
                    tokens::dark::ACCENT_DEFAULT
                } else {
                    tokens::dark::SURFACE_RAISED
                },
                fg: if on {
                    tokens::dark::TEXT_ON_ACCENT
                } else {
                    tokens::dark::TEXT_PRIMARY
                },
                border: Some(tokens::dark::BORDER_DEFAULT),
                radius: tokens::radius::MD,
                font_scale: 2,
                label,
                variant: if variant.is_empty() {
                    "switch".into()
                } else {
                    variant
                },
            }
        }
        "slider" => WidgetChrome {
            bg: tokens::dark::SURFACE_SUNKEN,
            fg: tokens::dark::ACCENT_DEFAULT,
            border: Some(tokens::dark::BORDER_DEFAULT),
            radius: tokens::radius::SM,
            font_scale: 2,
            label,
            variant: if variant.is_empty() {
                "range".into()
            } else {
                variant
            },
        },
        "dialog" => WidgetChrome {
            bg: tokens::dark::SURFACE_OVERLAY,
            fg: tokens::dark::TEXT_PRIMARY,
            border: Some(tokens::dark::BORDER_DEFAULT),
            radius: tokens::radius::LG,
            font_scale: 3,
            label,
            variant: "dialog".into(),
        },
        "icon" => WidgetChrome {
            bg: tokens::dark::SURFACE_RAISED,
            fg: tokens::dark::TEXT_PRIMARY,
            border: None,
            radius: tokens::radius::SM,
            font_scale: 1,
            label: String::new(),
            variant: if variant.is_empty() {
                "default".into()
            } else {
                variant
            },
        },
        "media" => WidgetChrome {
            bg: tokens::dark::SURFACE_SUNKEN,
            fg: tokens::dark::TEXT_PRIMARY,
            border: Some(tokens::dark::BORDER_DEFAULT),
            radius: tokens::radius::MD,
            font_scale: 3,
            label,
            variant: if variant.is_empty() {
                "player".into()
            } else {
                variant
            },
        },
        "chart" => WidgetChrome {
            bg: tokens::dark::SURFACE_CARD,
            fg: tokens::dark::ACCENT_DEFAULT,
            border: Some(tokens::dark::BORDER_DEFAULT),
            radius: tokens::radius::MD,
            font_scale: 2,
            label,
            variant: if variant.is_empty() {
                "bars".into()
            } else {
                variant
            },
        },
        _ => WidgetChrome {
            bg: tokens::dark::SURFACE_CARD,
            fg: tokens::dark::TEXT_PRIMARY,
            border: Some(tokens::dark::BORDER_DEFAULT),
            radius: tokens::radius::MD,
            font_scale: 2,
            label,
            variant,
        },
    }
}

/// Fill track + thumb geometry for a slider (paint hint for compositor).
pub fn slider_thumb_x(props: &Value, track_w: u32) -> u32 {
    let min = props.get("min").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let max = props
        .get("max")
        .and_then(|v| v.as_f64())
        .unwrap_or(100.0)
        .max(min + f64::EPSILON);
    let value = props
        .get("value")
        .and_then(|v| v.as_f64())
        .unwrap_or(min)
        .clamp(min, max);
    let t = (value - min) / (max - min);
    ((track_w.saturating_sub(16) as f64) * t) as u32
}

#[allow(dead_code)]
pub fn empty_props() -> Value {
    json!({})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_on_uses_accent() {
        let chrome = style(
            "toggle",
            "ui.wifi",
            &json!({ "checked": true, "label": "Wi-Fi" }),
        );
        assert_eq!(chrome.bg, tokens::dark::ACCENT_DEFAULT);
    }

    #[test]
    fn slider_thumb_scales() {
        let x = slider_thumb_x(&json!({ "min": 0, "max": 100, "value": 50 }), 200);
        assert!(x > 40 && x < 160, "x={x}");
    }
}
