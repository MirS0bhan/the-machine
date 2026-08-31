//! Design-system tokens for the Rust boot path.
//!
//! Canonical values from `docs/design-system/02-style/` (dark theme defaults).
//! Raw hex is allowed only here — AUIL/ASL consumers must reference tokens.

#![allow(dead_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub fn to_array(self) -> [u8; 3] {
        [self.0, self.1, self.2]
    }
}

pub fn parse_hex(hex: &str) -> Option<Rgb> {
    let h = hex.trim().trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some(Rgb(r, g, b))
}

/// Dark-theme token table (boot default).
pub mod dark {
    use super::Rgb;

    pub const SURFACE_CANVAS: Rgb = Rgb(0x0B, 0x0C, 0x13); // N950
    pub const SURFACE_SUNKEN: Rgb = Rgb(0x05, 0x05, 0x0A); // N1000
    pub const SURFACE_CARD: Rgb = Rgb(0x1C, 0x1E, 0x2B); // N800
    pub const SURFACE_RAISED: Rgb = Rgb(0x29, 0x2B, 0x3C); // N700
    pub const SURFACE_OVERLAY: Rgb = Rgb(0x29, 0x2B, 0x3C);

    pub const TEXT_PRIMARY: Rgb = Rgb(0xF7, 0xF8, 0xFC); // N10
    pub const TEXT_SECONDARY: Rgb = Rgb(0xA2, 0xA6, 0xBB); // N200
    pub const TEXT_TERTIARY: Rgb = Rgb(0x82, 0x86, 0x9C); // N300
    pub const TEXT_ON_ACCENT: Rgb = Rgb(0x12, 0x13, 0x1C); // N900

    pub const ACCENT_DEFAULT: Rgb = Rgb(0x9C, 0x7C, 0xF2);
    pub const ACCENT_HOVER: Rgb = Rgb(0xA9, 0x8E, 0xF5);
    pub const ACCENT_PRESSED: Rgb = Rgb(0x7E, 0x5F, 0xD6);
    pub const ACCENT_SUBTLE: Rgb = Rgb(0x2B, 0x1F, 0x4A);

    pub const BORDER_DEFAULT: Rgb = Rgb(0x3B, 0x3E, 0x52); // approx 12% of N10 over canvas
    pub const BORDER_FOCUS: Rgb = ACCENT_DEFAULT;

    pub const STATUS_DESTRUCTIVE: Rgb = Rgb(0xFF, 0x6B, 0x61);
    pub const CONFIRMATION_BG: Rgb = Rgb(0xC8, 0x50, 0x3C);
}

/// Space scale (`docs/design-system/02-style/01-design-tokens.md`).
pub mod space {
    pub const XS: u32 = 4;
    pub const SM: u32 = 8;
    pub const MD: u32 = 12;
    pub const LG: u32 = 16;
    pub const XL: u32 = 24;
    pub const XXL: u32 = 32;
    pub const XXXL: u32 = 48;
    pub const MIN_TARGET: u32 = 44;
}

/// Radius scale.
pub mod radius {
    pub const SM: u32 = 6;
    pub const MD: u32 = 10;
    pub const LG: u32 = 16;
    pub const XL: u32 = 24;
}

/// Type sizes in px (`docs/design-system/02-style/03-typography.md`).
pub mod type_size {
    pub const TITLE_2: u32 = 20;
    pub const BODY: u32 = 14;
    pub const CAPTION: u32 = 12;
    pub const LABEL: u32 = 13;
}

/// Light-theme token table.
pub mod light {
    use super::Rgb;

    pub const SURFACE_CANVAS: Rgb = Rgb(0xF7, 0xF8, 0xFC);
    pub const SURFACE_SUNKEN: Rgb = Rgb(0xEC, 0xEE, 0xF6);
    pub const SURFACE_CARD: Rgb = Rgb(0xFF, 0xFF, 0xFF);
    pub const SURFACE_RAISED: Rgb = Rgb(0xFF, 0xFF, 0xFF);
    pub const SURFACE_OVERLAY: Rgb = Rgb(0xE2, 0xE5, 0xF0);

    pub const TEXT_PRIMARY: Rgb = Rgb(0x12, 0x13, 0x1C);
    pub const TEXT_SECONDARY: Rgb = Rgb(0x4A, 0x4E, 0x63);
    pub const TEXT_TERTIARY: Rgb = Rgb(0x6B, 0x6F, 0x85);
    pub const TEXT_ON_ACCENT: Rgb = Rgb(0xFF, 0xFF, 0xFF);

    pub const ACCENT_DEFAULT: Rgb = Rgb(0x5B, 0x38, 0xD6);
    pub const ACCENT_HOVER: Rgb = Rgb(0x4C, 0x2C, 0xBE);
    pub const ACCENT_PRESSED: Rgb = Rgb(0x3E, 0x22, 0xA2);
    pub const ACCENT_SUBTLE: Rgb = Rgb(0xE7, 0xE0, 0xFB);

    pub const BORDER_DEFAULT: Rgb = Rgb(0xC5, 0xC9, 0xDA);
    pub const BORDER_FOCUS: Rgb = ACCENT_DEFAULT;

    pub const STATUS_DESTRUCTIVE: Rgb = Rgb(0xC0, 0x2B, 0x20);
    pub const CONFIRMATION_BG: Rgb = Rgb(0xA8, 0x38, 0x26);
}

/// High-contrast token table.
///
/// Pure black canvas with pure white text so every text/background pair clears
/// the WCAG AAA 7:1 ratio; `contrast_ratio` is asserted in tests.
pub mod high_contrast {
    use super::Rgb;

    pub const SURFACE_CANVAS: Rgb = Rgb(0x00, 0x00, 0x00);
    pub const SURFACE_SUNKEN: Rgb = Rgb(0x00, 0x00, 0x00);
    pub const SURFACE_CARD: Rgb = Rgb(0x00, 0x00, 0x00);
    pub const SURFACE_RAISED: Rgb = Rgb(0x00, 0x00, 0x00);
    pub const SURFACE_OVERLAY: Rgb = Rgb(0x00, 0x00, 0x00);

    pub const TEXT_PRIMARY: Rgb = Rgb(0xFF, 0xFF, 0xFF);
    pub const TEXT_SECONDARY: Rgb = Rgb(0xFF, 0xFF, 0xFF);
    pub const TEXT_TERTIARY: Rgb = Rgb(0xFF, 0xFF, 0xFF);
    pub const TEXT_ON_ACCENT: Rgb = Rgb(0x00, 0x00, 0x00);

    pub const ACCENT_DEFAULT: Rgb = Rgb(0xFF, 0xFF, 0x00);
    pub const ACCENT_HOVER: Rgb = Rgb(0xFF, 0xFF, 0x66);
    pub const ACCENT_PRESSED: Rgb = Rgb(0xCC, 0xCC, 0x00);
    pub const ACCENT_SUBTLE: Rgb = Rgb(0x33, 0x33, 0x00);

    pub const BORDER_DEFAULT: Rgb = Rgb(0xFF, 0xFF, 0xFF);
    pub const BORDER_FOCUS: Rgb = Rgb(0xFF, 0xFF, 0x00);

    pub const STATUS_DESTRUCTIVE: Rgb = Rgb(0xFF, 0x80, 0x80);
    pub const CONFIRMATION_BG: Rgb = Rgb(0x00, 0x00, 0x00);
}

/// Which token table paints the shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    Dark,
    Light,
    HighContrast,
}

impl Scheme {
    pub fn as_str(self) -> &'static str {
        match self {
            Scheme::Dark => "dark",
            Scheme::Light => "light",
            Scheme::HighContrast => "high-contrast",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "dark" => Some(Scheme::Dark),
            "light" => Some(Scheme::Light),
            "high-contrast" | "highcontrast" | "hc" | "contrast" => Some(Scheme::HighContrast),
            _ => None,
        }
    }
}

/// Resolved colors for the active scheme. Painting reads this, never a table
/// directly, so a scheme switch actually changes pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub scheme: Scheme,
    pub surface_canvas: Rgb,
    pub surface_sunken: Rgb,
    pub surface_card: Rgb,
    pub surface_raised: Rgb,
    pub surface_overlay: Rgb,
    pub text_primary: Rgb,
    pub text_secondary: Rgb,
    pub text_tertiary: Rgb,
    pub text_on_accent: Rgb,
    pub accent_default: Rgb,
    pub accent_hover: Rgb,
    pub accent_pressed: Rgb,
    pub accent_subtle: Rgb,
    pub border_default: Rgb,
    pub border_focus: Rgb,
    pub status_destructive: Rgb,
    pub confirmation_bg: Rgb,
}

macro_rules! palette_from {
    ($scheme:expr, $m:ident) => {
        Palette {
            scheme: $scheme,
            surface_canvas: $m::SURFACE_CANVAS,
            surface_sunken: $m::SURFACE_SUNKEN,
            surface_card: $m::SURFACE_CARD,
            surface_raised: $m::SURFACE_RAISED,
            surface_overlay: $m::SURFACE_OVERLAY,
            text_primary: $m::TEXT_PRIMARY,
            text_secondary: $m::TEXT_SECONDARY,
            text_tertiary: $m::TEXT_TERTIARY,
            text_on_accent: $m::TEXT_ON_ACCENT,
            accent_default: $m::ACCENT_DEFAULT,
            accent_hover: $m::ACCENT_HOVER,
            accent_pressed: $m::ACCENT_PRESSED,
            accent_subtle: $m::ACCENT_SUBTLE,
            border_default: $m::BORDER_DEFAULT,
            border_focus: $m::BORDER_FOCUS,
            status_destructive: $m::STATUS_DESTRUCTIVE,
            confirmation_bg: $m::CONFIRMATION_BG,
        }
    };
}

pub fn palette_for(scheme: Scheme) -> Palette {
    match scheme {
        Scheme::Dark => palette_from!(Scheme::Dark, dark),
        Scheme::Light => palette_from!(Scheme::Light, light),
        Scheme::HighContrast => palette_from!(Scheme::HighContrast, high_contrast),
    }
}

static SCHEME: std::sync::RwLock<Option<Scheme>> = std::sync::RwLock::new(None);

/// Boot default: dark, overridable with `THE_MACHINE_THEME`.
fn boot_scheme() -> Scheme {
    std::env::var("THE_MACHINE_THEME")
        .ok()
        .and_then(|v| Scheme::parse(&v))
        .unwrap_or(Scheme::Dark)
}

pub fn active_scheme() -> Scheme {
    SCHEME
        .read()
        .ok()
        .and_then(|g| *g)
        .unwrap_or_else(boot_scheme)
}

pub fn set_scheme(scheme: Scheme) {
    if let Ok(mut g) = SCHEME.write() {
        *g = Some(scheme);
    }
}

pub fn palette() -> Palette {
    palette_for(active_scheme())
}

/// Reduced motion / reduced transparency come from the same accessibility
/// preference block so `ui.theme.get` can report them honestly.
static REDUCED_MOTION: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static REDUCED_TRANSPARENCY: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

pub fn reduced_motion() -> bool {
    REDUCED_MOTION.load(std::sync::atomic::Ordering::Relaxed)
        || std::env::var("THE_MACHINE_REDUCED_MOTION").is_ok_and(|v| v != "0")
}

pub fn set_reduced_motion(on: bool) {
    REDUCED_MOTION.store(on, std::sync::atomic::Ordering::Relaxed);
}

pub fn reduced_transparency() -> bool {
    REDUCED_TRANSPARENCY.load(std::sync::atomic::Ordering::Relaxed)
        || active_scheme() == Scheme::HighContrast
        || std::env::var("THE_MACHINE_REDUCED_TRANSPARENCY").is_ok_and(|v| v != "0")
}

pub fn set_reduced_transparency(on: bool) {
    REDUCED_TRANSPARENCY.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// Active-palette accessors used by measure/style code.
///
/// Painting goes through these instead of a fixed table so `ui.theme.set` is a
/// real repaint rather than a stored preference nobody reads.
pub mod cur {
    use super::{palette, Rgb};

    macro_rules! accessor {
        ($name:ident, $field:ident) => {
            pub fn $name() -> Rgb {
                palette().$field
            }
        };
    }

    accessor!(surface_canvas, surface_canvas);
    accessor!(surface_sunken, surface_sunken);
    accessor!(surface_card, surface_card);
    accessor!(surface_raised, surface_raised);
    accessor!(surface_overlay, surface_overlay);
    accessor!(text_primary, text_primary);
    accessor!(text_secondary, text_secondary);
    accessor!(text_tertiary, text_tertiary);
    accessor!(text_on_accent, text_on_accent);
    accessor!(accent_default, accent_default);
    accessor!(accent_hover, accent_hover);
    accessor!(accent_pressed, accent_pressed);
    accessor!(accent_subtle, accent_subtle);
    accessor!(border_default, border_default);
    accessor!(border_focus, border_focus);
    accessor!(status_destructive, status_destructive);
    accessor!(confirmation_bg, confirmation_bg);
}

/// Relative luminance per WCAG 2.1.
fn relative_luminance(c: Rgb) -> f64 {
    fn chan(v: u8) -> f64 {
        let s = v as f64 / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * chan(c.0) + 0.7152 * chan(c.1) + 0.0722 * chan(c.2)
}

/// WCAG contrast ratio between two colors (1.0 … 21.0).
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

pub fn resolve_color(token: &str) -> Option<Rgb> {
    let p = palette();
    match token {
        "surface.canvas" => Some(p.surface_canvas),
        "surface.sunken" => Some(p.surface_sunken),
        "surface.card" => Some(p.surface_card),
        "surface.raised" => Some(p.surface_raised),
        "surface.overlay" => Some(p.surface_overlay),
        "text.primary" => Some(p.text_primary),
        "text.secondary" => Some(p.text_secondary),
        "text.tertiary" => Some(p.text_tertiary),
        "text.on-accent" | "accent.on-accent" => Some(p.text_on_accent),
        "accent.default" => Some(p.accent_default),
        "accent.hover" => Some(p.accent_hover),
        "accent.pressed" => Some(p.accent_pressed),
        "accent.subtle" => Some(p.accent_subtle),
        "border.default" => Some(p.border_default),
        "border.focus" => Some(p.border_focus),
        "status.destructive" => Some(p.status_destructive),
        _ => parse_hex(token),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        match LOCK.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }

    #[test]
    fn resolves_design_system_tokens() {
        let _g = guard();
        set_scheme(Scheme::Dark);
        assert_eq!(resolve_color("surface.canvas"), Some(dark::SURFACE_CANVAS));
        assert_eq!(resolve_color("accent.default"), Some(dark::ACCENT_DEFAULT));
        assert_eq!(parse_hex("#6C3CE0"), Some(Rgb(0x6C, 0x3C, 0xE0)));
    }

    #[test]
    fn scheme_names_round_trip() {
        assert_eq!(Scheme::parse("dark"), Some(Scheme::Dark));
        assert_eq!(Scheme::parse("Light"), Some(Scheme::Light));
        assert_eq!(Scheme::parse("high_contrast"), Some(Scheme::HighContrast));
        assert_eq!(Scheme::parse("hc"), Some(Scheme::HighContrast));
        assert!(Scheme::parse("neon").is_none());
        assert_eq!(Scheme::HighContrast.as_str(), "high-contrast");
    }

    #[test]
    fn switching_scheme_changes_resolved_colors() {
        let _g = guard();
        set_scheme(Scheme::Light);
        assert_eq!(resolve_color("surface.canvas"), Some(light::SURFACE_CANVAS));
        set_scheme(Scheme::HighContrast);
        assert_eq!(resolve_color("text.primary"), Some(Rgb(0xFF, 0xFF, 0xFF)));
        set_scheme(Scheme::Dark);
    }

    #[test]
    fn every_scheme_meets_text_contrast_targets() {
        for (scheme, min) in [
            (Scheme::Dark, 4.5),
            (Scheme::Light, 4.5),
            (Scheme::HighContrast, 7.0),
        ] {
            let p = palette_for(scheme);
            for (name, fg, bg) in [
                ("primary/canvas", p.text_primary, p.surface_canvas),
                ("secondary/canvas", p.text_secondary, p.surface_canvas),
                ("primary/card", p.text_primary, p.surface_card),
                ("on-accent/accent", p.text_on_accent, p.accent_default),
            ] {
                let ratio = contrast_ratio(fg, bg);
                assert!(
                    ratio >= min,
                    "{} {name} contrast {ratio:.2} below {min}",
                    scheme.as_str()
                );
            }
        }
    }

    #[test]
    fn high_contrast_forces_opaque_surfaces() {
        let _g = guard();
        set_scheme(Scheme::HighContrast);
        assert!(reduced_transparency());
        set_scheme(Scheme::Dark);
        set_reduced_transparency(false);
        assert!(!reduced_transparency());
    }

    #[test]
    fn reduced_motion_is_off_by_default_and_settable() {
        let _g = guard();
        set_reduced_motion(false);
        assert!(!reduced_motion());
        set_reduced_motion(true);
        assert!(reduced_motion());
        set_reduced_motion(false);
    }

    #[test]
    fn contrast_ratio_bounds() {
        assert!((contrast_ratio(Rgb(0, 0, 0), Rgb(255, 255, 255)) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(Rgb(9, 9, 9), Rgb(9, 9, 9)) - 1.0).abs() < 0.001);
    }
}
