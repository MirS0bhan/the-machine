use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Optional RGB triple carried on the bus as `[r, g, b]`.
pub type Rgb = [u8; 3];

#[derive(Clone, Serialize, Deserialize)]
pub struct Surface {
    pub id: String,
    #[serde(default)]
    pub parent: Option<String>,
    #[serde(default)]
    pub children: Vec<String>,
    #[serde(default)]
    pub geometry: Geometry,
    #[serde(default)]
    pub z_order: i32,
    #[serde(default = "default_one")]
    pub opacity: f32,
    /// Motion target opacity (present-loop lerps `opacity` toward this).
    #[serde(default = "default_one")]
    pub opacity_target: f32,
    /// Motion duration hint in ms (snappy≈120, gentle≈280).
    #[serde(default = "default_motion_ms")]
    pub motion_ms: u32,
    #[serde(default)]
    pub blurred: bool,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub focused: bool,
    /// Local press feedback (pointer down) — painted without agent round-trip.
    #[serde(default)]
    pub pressed: bool,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub confirmation: bool,
    /// Toggle checked state.
    #[serde(default)]
    pub checked: bool,
    /// Slider / progress value in [0, 1] (normalized) or absolute when `value_max` > 0.
    #[serde(default)]
    pub value: f64,
    #[serde(default)]
    pub value_min: f64,
    #[serde(default)]
    pub value_max: f64,
    /// Vertical scroll offset for list / overflow containers.
    #[serde(default)]
    pub scroll_y: i32,
    /// List row labels (newline-joined fallback lives in `label`).
    #[serde(default)]
    pub items: Vec<String>,
    /// Caret byte offset for focused fields (−1 = hidden).
    #[serde(default = "default_caret")]
    pub caret: i64,
    /// When true, `label` is placeholder chrome — caret paints at start.
    #[serde(default)]
    pub placeholder_active: bool,
    /// Fill color (design-system token resolved by ui-runtime). Absent → kind fallback.
    #[serde(default)]
    pub bg: Option<Rgb>,
    /// Label / content color.
    #[serde(default)]
    pub fg: Option<Rgb>,
    /// Optional 1px border color (fields, cards).
    #[serde(default)]
    pub border: Option<Rgb>,
    /// Corner radius in px (`radius.md` = 10 for controls).
    #[serde(default)]
    pub radius: u32,
    /// Bitmap font pixel scale (legacy). Prefer `font_px`.
    #[serde(default = "default_font_scale")]
    pub font_scale: u32,
    /// Explicit pixel size from design-system type scale (title-2=20, body=14, …).
    #[serde(default)]
    pub font_px: u32,
    /// `regular` | `medium` | `bold`
    #[serde(default = "default_font_weight")]
    pub font_weight: String,
    /// `default` (Inter) | `numeric` (JetBrains Mono)
    #[serde(default = "default_font_family")]
    pub font_family: String,
    /// Visual variant (`primary`, `field`, …).
    #[serde(default)]
    pub variant: String,
}

fn default_one() -> f32 {
    1.0
}

fn default_font_scale() -> u32 {
    2
}

fn default_font_weight() -> String {
    "regular".into()
}

fn default_font_family() -> String {
    "default".into()
}

fn default_caret() -> i64 {
    -1
}

fn default_motion_ms() -> u32 {
    120
}

pub struct Compositor {
    pub surfaces: HashMap<String, Surface>,
    pub order: Vec<String>,
    pub focused: Option<String>,
    pub confirmation_active: bool,
    pub confirmation_surface: Option<String>,
    /// Soft modal exclusivity for generic `dialog` (not confirmation/e4).
    pub dialog_active: bool,
    pub dialog_surface: Option<String>,
    pub damage: crate::damage::DamageTracker,
}

impl Compositor {
    pub fn new() -> Self {
        Compositor {
            surfaces: HashMap::new(),
            order: Vec::new(),
            focused: None,
            confirmation_active: false,
            confirmation_surface: None,
            dialog_active: false,
            dialog_surface: None,
            damage: crate::damage::DamageTracker::new(),
        }
    }

    pub fn recompute_order(&mut self) {
        let mut v: Vec<&String> = self.surfaces.keys().collect();
        v.sort_by_key(|id| self.surfaces[*id].z_order);
        self.order = v.into_iter().cloned().collect();
    }

    pub fn pick(&self, x: i32, y: i32) -> Option<String> {
        if self.confirmation_active {
            if let Some(ref sid) = self.confirmation_surface {
                if self.hit(sid, x, y) {
                    return Some(sid.clone());
                }
            }
            return None;
        }
        if self.dialog_active {
            if let Some(ref sid) = self.dialog_surface {
                if self.hit(sid, x, y) {
                    return Some(sid.clone());
                }
            }
            // Allow hitting dialog children that share the dialog id prefix.
            for id in self.order.iter().rev() {
                if self.hit(id, x, y) {
                    if let Some(s) = self.surfaces.get(id) {
                        if s.kind == "dialog"
                            || self
                                .dialog_surface
                                .as_ref()
                                .is_some_and(|d| id.starts_with(d) || d.starts_with(id))
                        {
                            return Some(id.clone());
                        }
                    }
                }
            }
            return None;
        }
        for id in self.order.iter().rev() {
            if self.hit(id, x, y) {
                return Some(id.clone());
            }
        }
        None
    }

    fn hit(&self, id: &str, x: i32, y: i32) -> bool {
        self.surfaces.get(id).is_some_and(|s| {
            let g = &s.geometry;
            x >= g.x && x < g.x + g.width as i32 && y >= g.y && y < g.y + g.height as i32
        })
    }
}
