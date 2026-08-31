//! Drag-and-drop helpers (P2 event path).

use serde_json::{json, Value};

#[derive(Clone, Debug, Default)]
pub struct DragSession {
    pub source_id: String,
    pub active: bool,
    pub start_x: i32,
    pub start_y: i32,
    pub x: i32,
    pub y: i32,
    pub payload: Value,
}

impl DragSession {
    pub fn begin(source_id: &str, x: i32, y: i32, payload: Value) -> Self {
        DragSession {
            source_id: source_id.to_string(),
            active: true,
            start_x: x,
            start_y: y,
            x,
            y,
            payload,
        }
    }

    pub fn move_to(&mut self, x: i32, y: i32) {
        self.x = x;
        self.y = y;
    }

    pub fn delta(&self) -> (i32, i32) {
        (self.x - self.start_x, self.y - self.start_y)
    }

    pub fn end_payload(&self, target_id: Option<&str>) -> Value {
        let (dx, dy) = self.delta();
        json!({
            "source": self.source_id,
            "target": target_id,
            "x": self.x,
            "y": self.y,
            "dx": dx,
            "dy": dy,
            "payload": self.payload,
        })
    }
}

pub fn is_draggable(props: &std::collections::HashMap<String, Value>) -> bool {
    props
        .get("draggable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || props
            .get("drag")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_delta() {
        let mut d = DragSession::begin("a", 10, 10, json!({}));
        d.move_to(25, 12);
        assert_eq!(d.delta(), (15, 2));
    }
}
