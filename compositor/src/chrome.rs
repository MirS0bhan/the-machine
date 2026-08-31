//! Design-system chrome tokens for the compositor paint path.
//!
//! Values mirror `docs/design-system/02-style/` dark theme (and
//! `ui-runtime/src/tokens.rs`). Raw hex stays here — bus clients pass
//! resolved RGB on surfaces.

#![allow(dead_code)]

pub const SURFACE_CANVAS: [u8; 3] = [0x0B, 0x0C, 0x13];
pub const SURFACE_SUNKEN: [u8; 3] = [0x05, 0x05, 0x0A];
pub const SURFACE_CARD: [u8; 3] = [0x1C, 0x1E, 0x2B];
pub const SURFACE_RAISED: [u8; 3] = [0x29, 0x2B, 0x3C];
pub const SURFACE_OVERLAY: [u8; 3] = [0x29, 0x2B, 0x3C];

pub const TEXT_PRIMARY: [u8; 3] = [0xF7, 0xF8, 0xFC];
pub const TEXT_SECONDARY: [u8; 3] = [0xA2, 0xA6, 0xBB];
pub const TEXT_TERTIARY: [u8; 3] = [0x82, 0x86, 0x9C];
pub const TEXT_ON_ACCENT: [u8; 3] = [0x12, 0x13, 0x1C];

pub const ACCENT_DEFAULT: [u8; 3] = [0x9C, 0x7C, 0xF2];
pub const BORDER_DEFAULT: [u8; 3] = [0x3B, 0x3E, 0x52];
pub const BORDER_FOCUS: [u8; 3] = ACCENT_DEFAULT;
pub const CONFIRMATION_BG: [u8; 3] = [0xC8, 0x50, 0x3C];

pub const RADIUS_MD: u32 = 10;
pub const RADIUS_LG: u32 = 16;
