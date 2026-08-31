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

pub fn resolve_color(token: &str) -> Option<Rgb> {
    match token {
        "surface.canvas" => Some(dark::SURFACE_CANVAS),
        "surface.sunken" => Some(dark::SURFACE_SUNKEN),
        "surface.card" => Some(dark::SURFACE_CARD),
        "surface.raised" => Some(dark::SURFACE_RAISED),
        "surface.overlay" => Some(dark::SURFACE_OVERLAY),
        "text.primary" => Some(dark::TEXT_PRIMARY),
        "text.secondary" => Some(dark::TEXT_SECONDARY),
        "text.tertiary" => Some(dark::TEXT_TERTIARY),
        "text.on-accent" | "accent.on-accent" => Some(dark::TEXT_ON_ACCENT),
        "accent.default" => Some(dark::ACCENT_DEFAULT),
        "accent.hover" => Some(dark::ACCENT_HOVER),
        "accent.subtle" => Some(dark::ACCENT_SUBTLE),
        "border.default" => Some(dark::BORDER_DEFAULT),
        "border.focus" => Some(dark::BORDER_FOCUS),
        "status.destructive" => Some(dark::STATUS_DESTRUCTIVE),
        _ => parse_hex(token),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_design_system_tokens() {
        assert_eq!(resolve_color("surface.canvas"), Some(dark::SURFACE_CANVAS));
        assert_eq!(resolve_color("accent.default"), Some(dark::ACCENT_DEFAULT));
        assert_eq!(parse_hex("#6C3CE0"), Some(Rgb(0x6C, 0x3C, 0xE0)));
    }
}
