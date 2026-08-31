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
        let offset_y = props.get("scroll_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
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

/// Row height assumed for list content when the node does not declare one.
pub const ROW_H: u32 = 32;

/// Fraction of momentum retained per settle tick (kinetic decay).
pub const KINETIC_DECAY: f64 = 0.82;

/// Velocity below which momentum is considered spent.
pub const KINETIC_CUTOFF: f64 = 1.0;

fn content_height(props: &std::collections::HashMap<String, Value>, viewport_h: u32) -> u32 {
    if let Some(h) = props.get("content_h").and_then(|v| v.as_u64()) {
        return h as u32;
    }
    // Derive from item count so a plain `items` list scrolls without extra props.
    let rows = props
        .get("items")
        .or_else(|| props.get("data"))
        .and_then(|v| v.as_array())
        .map(|a| a.len() as u32)
        .unwrap_or(0);
    let row_h = props
        .get("row_h")
        .and_then(|v| v.as_u64())
        .unwrap_or(ROW_H as u64) as u32;
    (rows * row_h).max(viewport_h)
}

fn state_from(props: &std::collections::HashMap<String, Value>, viewport_h: u32) -> ScrollState {
    ScrollState {
        offset_y: props.get("scroll_y").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        content_h: content_height(props, viewport_h),
        viewport_h,
    }
}

fn commit(props: &mut std::collections::HashMap<String, Value>, state: &ScrollState) {
    props.insert("scroll_y".into(), serde_json::json!(state.offset_y));
    props.insert("scroll_max".into(), serde_json::json!(state.max_offset()));
    props.insert("content_h".into(), serde_json::json!(state.content_h));
}

/// Apply a wheel delta to a node's scroll_y prop (mutates props map via JSON).
///
/// The delta also seeds `scroll_velocity`, which `settle` decays so a flick
/// keeps gliding instead of stopping dead on the last event.
pub fn apply_wheel(
    props: &mut std::collections::HashMap<String, Value>,
    delta_y: i32,
    viewport_h: u32,
) {
    let mut state = state_from(props, viewport_h);
    state.scroll_by(delta_y);
    let prior = props
        .get("scroll_velocity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    // Same-direction flicks accumulate momentum; a reverse flick cancels it.
    let velocity = if prior.signum() == (delta_y as f64).signum() {
        prior * KINETIC_DECAY + delta_y as f64
    } else {
        delta_y as f64
    };
    commit(props, &state);
    props.insert("scroll_velocity".into(), serde_json::json!(velocity));
}

/// Advance kinetic momentum one tick. Returns true while still gliding.
pub fn settle(props: &mut std::collections::HashMap<String, Value>, viewport_h: u32) -> bool {
    let velocity = props
        .get("scroll_velocity")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    if velocity.abs() < KINETIC_CUTOFF {
        props.insert("scroll_velocity".into(), serde_json::json!(0.0));
        return false;
    }
    let mut state = state_from(props, viewport_h);
    let before = state.offset_y;
    state.scroll_by(velocity.round() as i32);
    let next = velocity * KINETIC_DECAY;
    commit(props, &state);
    // Hitting an edge ends the glide rather than spinning on a clamped offset.
    let at_edge = state.offset_y == before;
    let remaining = if at_edge || next.abs() < KINETIC_CUTOFF {
        0.0
    } else {
        next
    };
    props.insert("scroll_velocity".into(), serde_json::json!(remaining));
    remaining.abs() >= KINETIC_CUTOFF
}

/// Scroll by whole pages (PageUp / PageDown).
pub fn apply_page(
    props: &mut std::collections::HashMap<String, Value>,
    pages: i32,
    viewport_h: u32,
) {
    let mut state = state_from(props, viewport_h);
    state.scroll_by(pages * viewport_h.max(1) as i32);
    commit(props, &state);
    props.insert("scroll_velocity".into(), serde_json::json!(0.0));
}

/// Scroll to an absolute offset (`ui.scroll`).
pub fn scroll_to(
    props: &mut std::collections::HashMap<String, Value>,
    offset_y: i32,
    viewport_h: u32,
) {
    let mut state = state_from(props, viewport_h);
    state.offset_y = offset_y;
    state.clamp();
    commit(props, &state);
    props.insert("scroll_velocity".into(), serde_json::json!(0.0));
}

/// Move the highlighted row of a list and keep it inside the viewport.
///
/// Returns the new selected index.
pub fn move_selection(
    props: &mut std::collections::HashMap<String, Value>,
    delta: i32,
    viewport_h: u32,
) -> usize {
    let count = props
        .get("items")
        .or_else(|| props.get("data"))
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    if count == 0 {
        return 0;
    }
    let current = props.get("selected").and_then(|v| v.as_i64()).unwrap_or(0);
    let next = (current + delta as i64).clamp(0, count as i64 - 1) as usize;
    props.insert("selected".into(), serde_json::json!(next));
    ensure_visible(props, next, viewport_h);
    next
}

/// Set selection to an absolute index (Home / End / click).
pub fn set_selection(
    props: &mut std::collections::HashMap<String, Value>,
    index: usize,
    viewport_h: u32,
) -> usize {
    let count = props
        .get("items")
        .or_else(|| props.get("data"))
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    if count == 0 {
        return 0;
    }
    let next = index.min(count - 1);
    props.insert("selected".into(), serde_json::json!(next));
    ensure_visible(props, next, viewport_h);
    next
}

/// Scroll the minimum amount needed to bring row `index` into view.
pub fn ensure_visible(
    props: &mut std::collections::HashMap<String, Value>,
    index: usize,
    viewport_h: u32,
) {
    let row_h = props
        .get("row_h")
        .and_then(|v| v.as_u64())
        .unwrap_or(ROW_H as u64) as u32;
    let top = (index as u32 * row_h) as i32;
    let bottom = top + row_h as i32;
    let mut state = state_from(props, viewport_h);
    if top < state.offset_y {
        state.offset_y = top;
    } else if bottom > state.offset_y + viewport_h as i32 {
        state.offset_y = bottom - viewport_h as i32;
    }
    state.clamp();
    commit(props, &state);
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

    fn list_props(items: usize) -> std::collections::HashMap<String, Value> {
        let mut props = std::collections::HashMap::new();
        let arr: Vec<Value> = (0..items)
            .map(|i| serde_json::json!(format!("row {i}")))
            .collect();
        props.insert("items".into(), serde_json::json!(arr));
        props
    }

    #[test]
    fn content_height_derives_from_items() {
        let props = list_props(20);
        assert_eq!(content_height(&props, 100), 20 * ROW_H);
    }

    #[test]
    fn kinetic_flick_glides_then_stops() {
        let mut props = list_props(40);
        apply_wheel(&mut props, 60, 160);
        assert!(
            props
                .get("scroll_velocity")
                .and_then(|v| v.as_f64())
                .unwrap()
                >= 60.0
        );
        let mut ticks = 0;
        while settle(&mut props, 160) {
            ticks += 1;
            assert!(ticks < 50, "momentum must decay");
        }
        assert!(ticks > 0, "a flick should glide at least one tick");
        assert_eq!(
            props.get("scroll_velocity").and_then(|v| v.as_f64()),
            Some(0.0)
        );
    }

    #[test]
    fn same_direction_flicks_accumulate_momentum() {
        let mut props = list_props(80);
        apply_wheel(&mut props, 40, 160);
        let first = props["scroll_velocity"].as_f64().unwrap();
        apply_wheel(&mut props, 40, 160);
        let second = props["scroll_velocity"].as_f64().unwrap();
        assert!(second > first, "{second} should exceed {first}");
    }

    #[test]
    fn reverse_flick_cancels_momentum() {
        let mut props = list_props(80);
        apply_wheel(&mut props, 40, 160);
        apply_wheel(&mut props, -40, 160);
        assert_eq!(props["scroll_velocity"].as_f64(), Some(-40.0));
    }

    #[test]
    fn page_scroll_moves_by_viewport() {
        let mut props = list_props(40);
        apply_page(&mut props, 1, 160);
        assert_eq!(props["scroll_y"].as_i64(), Some(160));
        apply_page(&mut props, -1, 160);
        assert_eq!(props["scroll_y"].as_i64(), Some(0));
    }

    #[test]
    fn scroll_to_clamps() {
        let mut props = list_props(10);
        scroll_to(&mut props, 99999, 160);
        assert_eq!(props["scroll_y"].as_i64(), Some((10 * ROW_H - 160) as i64));
    }

    #[test]
    fn selection_moves_and_clamps() {
        let mut props = list_props(5);
        assert_eq!(move_selection(&mut props, 1, 160), 1);
        assert_eq!(move_selection(&mut props, 99, 160), 4);
        assert_eq!(move_selection(&mut props, -99, 160), 0);
    }

    #[test]
    fn selection_scrolls_row_into_view() {
        let mut props = list_props(40);
        set_selection(&mut props, 30, 160);
        let offset = props["scroll_y"].as_i64().unwrap();
        let row_top = 30 * ROW_H as i64;
        assert!(
            offset <= row_top && row_top < offset + 160,
            "offset {offset}"
        );
    }

    #[test]
    fn empty_list_selection_is_noop() {
        let mut props = std::collections::HashMap::new();
        assert_eq!(move_selection(&mut props, 1, 160), 0);
    }
}
