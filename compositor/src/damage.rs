//! Dirty-rect tracking for incremental presents.

use crate::model::Geometry;

#[derive(Clone, Debug, Default)]
pub struct DamageTracker {
    rects: Vec<Geometry>,
    full: bool,
}

impl DamageTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mark_full(&mut self) {
        self.full = true;
        self.rects.clear();
    }

    pub fn add(&mut self, g: Geometry) {
        if self.full || g.width == 0 || g.height == 0 {
            return;
        }
        self.rects.push(g);
        if self.rects.len() > 32 {
            self.mark_full();
        }
    }

    pub fn add_union(&mut self, a: &Geometry, b: &Geometry) {
        self.add(a.clone());
        self.add(b.clone());
    }

    pub fn take(&mut self) -> DamageFrame {
        let full = self.full;
        let rects = std::mem::take(&mut self.rects);
        self.full = false;
        DamageFrame { full, rects }
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        !self.full && self.rects.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct DamageFrame {
    pub full: bool,
    pub rects: Vec<Geometry>,
}

impl DamageFrame {
    pub fn union_bounds(&self) -> Option<Geometry> {
        if self.full || self.rects.is_empty() {
            return None;
        }
        let mut min_x = i32::MAX;
        let mut min_y = i32::MAX;
        let mut max_x = i32::MIN;
        let mut max_y = i32::MIN;
        for g in &self.rects {
            min_x = min_x.min(g.x);
            min_y = min_y.min(g.y);
            max_x = max_x.max(g.x + g.width as i32);
            max_y = max_y.max(g.y + g.height as i32);
        }
        Some(Geometry {
            x: min_x,
            y: min_y,
            width: (max_x - min_x).max(0) as u32,
            height: (max_y - min_y).max(0) as u32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unions_rects() {
        let mut d = DamageTracker::new();
        d.add(Geometry {
            x: 10,
            y: 10,
            width: 20,
            height: 20,
        });
        d.add(Geometry {
            x: 40,
            y: 40,
            width: 10,
            height: 10,
        });
        let frame = d.take();
        let b = frame.union_bounds().unwrap();
        assert_eq!(b.x, 10);
        assert_eq!(b.y, 10);
        assert_eq!(b.width, 40);
        assert_eq!(b.height, 40);
    }
}
