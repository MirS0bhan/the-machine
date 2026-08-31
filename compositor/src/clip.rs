//! Clip / scissor helpers for scroll views (P1 framework breadth).

use crate::pixel::PixelBackend;

/// Inclusive rectangle used as a software scissor.
#[derive(Clone, Copy, Debug, Default)]
pub struct ClipRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl ClipRect {
    #[allow(dead_code)]
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x
            && py >= self.y
            && px < self.x + self.width as i32
            && py < self.y + self.height as i32
    }

    pub fn intersect(self, other: ClipRect) -> Option<ClipRect> {
        let x0 = self.x.max(other.x);
        let y0 = self.y.max(other.y);
        let x1 = (self.x + self.width as i32).min(other.x + other.width as i32);
        let y1 = (self.y + self.height as i32).min(other.y + other.height as i32);
        if x1 <= x0 || y1 <= y0 {
            None
        } else {
            Some(ClipRect {
                x: x0,
                y: y0,
                width: (x1 - x0) as u32,
                height: (y1 - y0) as u32,
            })
        }
    }
}

/// Fill a rect intersected with an optional clip.
#[allow(dead_code)]
pub fn fill_rect_clipped(
    px: &mut PixelBackend,
    clip: Option<ClipRect>,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    rgb: [u8; 3],
) {
    let rect = ClipRect {
        x,
        y,
        width: w,
        height: h,
    };
    let drawn = match clip {
        Some(c) => match c.intersect(rect) {
            Some(r) => r,
            None => return,
        },
        None => rect,
    };
    px.fill_rect(drawn.x, drawn.y, drawn.width, drawn.height, rgb);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intersect_overlap() {
        let a = ClipRect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let b = ClipRect {
            x: 50,
            y: 50,
            width: 100,
            height: 100,
        };
        let i = a.intersect(b).unwrap();
        assert_eq!(i.x, 50);
        assert_eq!(i.width, 50);
    }
}
