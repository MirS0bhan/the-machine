use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

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
    #[serde(default)]
    pub blurred: bool,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub focused: bool,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub confirmation: bool,
}

fn default_one() -> f32 {
    1.0
}

pub struct Compositor {
    pub surfaces: HashMap<String, Surface>,
    pub order: Vec<String>,
    pub focused: Option<String>,
    pub confirmation_active: bool,
    pub confirmation_surface: Option<String>,
}

impl Compositor {
    pub fn new() -> Self {
        Compositor {
            surfaces: HashMap::new(),
            order: Vec::new(),
            focused: None,
            confirmation_active: false,
            confirmation_surface: None,
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
