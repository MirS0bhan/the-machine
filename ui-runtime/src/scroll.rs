//! Scroll offset helpers for list / overflow containers.

use serde_json::Value;

#[derive(Clone, Debug, Default)]
pub struct ScrollState {
    pub offset_y: i32,
    pub content_h: u32,
    pub viewport_h: u32,
}

impl ScrollState {
    #[allow(dead_code)]
    pub fn from_props(props: &Value, viewport_h: u32, content_h: u32) -> Self {
        let offset_y = props
            .get("scroll_y")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let mut s = ScrollState {
            offset_y,
            content_h,
            viewport_h,
        };
        s.clamp();
        s
    }

    pub fn max_offset(&self) -> i32 {
        self.content_h.saturating_sub(self.viewport_h) as i32
    }

    pub fn clamp(&mut self) {
        let max = self.max_offset();
        if self.offset_y < 0 {
            self.offset_y = 0;
        } else if self.offset_y > max {
            self.offset_y = max;
        }
    }

    pub fn scroll_by(&mut self, dy: i32) {
        self.offset_y += dy;
        self.clamp();
    }
}

/// Apply a wheel delta to a node's scroll_y prop (mutates props map via JSON).
pub fn apply_wheel(
    props: &mut std::collections::HashMap<String, Value>,
    delta_y: i32,
    viewport_h: u32,
) {
    let content_h = props
        .get("content_h")
        .and_then(|v| v.as_u64())
        .unwrap_or(viewport_h as u64) as u32;
    let offset_y = props
        .get("scroll_y")
        .and_then(|v| v.as_i64())
        .unwrap_or(0) as i32;
    let mut state = ScrollState {
        offset_y,
        content_h,
        viewport_h,
    };
    state.scroll_by(delta_y);
    props.insert("scroll_y".into(), serde_json::json!(state.offset_y));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_to_content() {
        let mut s = ScrollState {
            offset_y: 500,
            content_h: 200,
            viewport_h: 100,
        };
        s.clamp();
        assert_eq!(s.offset_y, 100);
    }

    #[test]
    fn wheel_updates_prop() {
        let mut props = std::collections::HashMap::new();
        props.insert("content_h".into(), serde_json::json!(400));
        apply_wheel(&mut props, 40, 100);
        assert_eq!(props.get("scroll_y").and_then(|v| v.as_i64()), Some(40));
    }
}
