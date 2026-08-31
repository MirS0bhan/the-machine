//! Motion curves + tween helpers (P2 present-loop friendly).

use serde_json::Value;

#[derive(Clone, Copy, Debug)]
pub struct MotionCurve {
    pub name: &'static str,
    pub duration_ms: u32,
    /// 0 = linear, higher = ease-out bias.
    pub ease: f32,
}

pub const SNAPPY: MotionCurve = MotionCurve {
    name: "snappy",
    duration_ms: 120,
    ease: 2.0,
};
pub const GENTLE: MotionCurve = MotionCurve {
    name: "gentle",
    duration_ms: 280,
    ease: 1.4,
};
pub const REDUCED: MotionCurve = MotionCurve {
    name: "reduced",
    duration_ms: 1,
    ease: 1.0,
};

pub fn curve_named(name: &str) -> MotionCurve {
    match name {
        "gentle" => GENTLE,
        "reduced" | "motion.reduced" => REDUCED,
        _ => SNAPPY,
    }
}

/// Ease-out progress in [0, 1].
pub fn ease_out(t: f32, power: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powf(power.max(0.1))
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// Read preferred motion from props / prefers-reduced-motion style flag.
pub fn curve_for_props(props: &Value) -> MotionCurve {
    if props
        .get("reduced_motion")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return REDUCED;
    }
    let name = props
        .get("motion")
        .and_then(|v| v.as_str())
        .unwrap_or("snappy");
    curve_named(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_ends_at_one() {
        assert!((ease_out(1.0, 2.0) - 1.0).abs() < f32::EPSILON);
        assert!(ease_out(0.0, 2.0).abs() < f32::EPSILON);
    }
}
