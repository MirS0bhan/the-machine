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

/// Motion requested by a node's props, if any (`motion=gentle`).
pub fn requested_curve(props: &Value) -> Option<String> {
    if props
        .get("reduced_motion")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Some("reduced".into());
    }
    props
        .get("motion")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Curve for a laid-out node, resolved for the compositor present loop.
///
/// The global accessibility preference wins over a per-node request, so turning
/// reduced motion on cannot be overridden by a node that asks for `gentle`.
pub fn curve_for_node(kind: &str, requested: &str) -> MotionCurve {
    if crate::tokens::reduced_motion() {
        return REDUCED;
    }
    if !requested.is_empty() {
        return curve_named(requested);
    }
    match kind {
        // Overlays get the longer curve; ordinary controls stay snappy.
        "dialog" | "media" => GENTLE,
        _ => SNAPPY,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ease_ends_at_one() {
        assert!((ease_out(1.0, 2.0) - 1.0).abs() < f32::EPSILON);
        assert!(ease_out(0.0, 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn node_request_selects_curve() {
        crate::tokens::set_reduced_motion(false);
        assert_eq!(curve_for_node("button", "").name, "snappy");
        assert_eq!(curve_for_node("dialog", "").name, "gentle");
        assert_eq!(curve_for_node("button", "gentle").name, "gentle");
    }

    #[test]
    fn reduced_motion_preference_overrides_node_request() {
        crate::tokens::set_reduced_motion(true);
        let c = curve_for_node("dialog", "gentle");
        assert_eq!(c.name, "reduced");
        assert_eq!(c.duration_ms, 1);
        crate::tokens::set_reduced_motion(false);
    }

    #[test]
    fn props_request_is_read_from_the_node() {
        assert_eq!(
            requested_curve(&serde_json::json!({ "motion": "gentle" })).as_deref(),
            Some("gentle")
        );
        assert_eq!(
            requested_curve(&serde_json::json!({ "reduced_motion": true })).as_deref(),
            Some("reduced")
        );
        assert!(requested_curve(&serde_json::json!({})).is_none());
    }
}
