//! HarfRust (HarfBuzz algorithm port) + FreeType text for compositor chrome.
//!
//! Pipeline: Unicode → HarfRust shape → FreeType glyph bitmaps → BGRA blit.
//! Falls back to the 5×7 bitmap font only when no Inter face can be loaded.

use crate::bitmap_font;
use crate::pixel::PixelBackend;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontWeight {
    Regular,
    Medium,
    Bold,
}

impl FontWeight {
    pub fn parse(s: &str) -> Self {
        match s {
            "medium" | "500" => Self::Medium,
            "bold" | "700" => Self::Bold,
            _ => Self::Regular,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
    /// `type.family.default` — Inter
    Default,
    /// `type.family.numeric` — JetBrains Mono
    Numeric,
}

impl FontFamily {
    pub fn parse(s: &str) -> Self {
        match s {
            "numeric" | "mono" | "type.family.numeric" => Self::Numeric,
            _ => Self::Default,
        }
    }
}

struct FaceBlob {
    path: PathBuf,
    /// Owned font bytes — `FontRef` borrows these for shaping.
    bytes: Vec<u8>,
}

struct Engine {
    ft: freetype::Library,
    faces: HashMap<(FontFamily, FontWeight), FaceBlob>,
    glyph_cache: HashMap<(FontFamily, FontWeight, u32, u32), CachedGlyph>,
    backend: &'static str,
}

// FreeType Library is !Send; access is serialized by ENGINE's Mutex.
unsafe impl Send for Engine {}

struct CachedGlyph {
    width: i32,
    rows: i32,
    left: i32,
    top: i32,
    buffer: Vec<u8>,
}

static ENGINE: OnceLock<Mutex<Option<Engine>>> = OnceLock::new();

fn engine() -> &'static Mutex<Option<Engine>> {
    ENGINE.get_or_init(|| Mutex::new(Engine::try_open()))
}

impl Engine {
    fn try_open() -> Option<Self> {
        let ft = freetype::Library::init().ok()?;
        let mut faces = HashMap::new();
        for (family, weight, candidates) in face_candidates() {
            if let Some(path) = candidates.into_iter().find(|p| Path::new(p).exists()) {
                if let Ok(bytes) = std::fs::read(&path) {
                    // Validate FreeType + HarfRust can open it.
                    if ft.new_face(&path, 0).is_ok() && harfrust::FontRef::new(&bytes).is_ok() {
                        faces.insert(
                            (family, weight),
                            FaceBlob {
                                path: PathBuf::from(path),
                                bytes,
                            },
                        );
                    }
                }
            }
        }
        if faces.is_empty() {
            eprintln!("[compositor] text: no Inter/JetBrains faces found — bitmap fallback");
            return None;
        }
        eprintln!(
            "[compositor] text: HarfRust+FreeType active ({} faces)",
            faces.len()
        );
        Some(Engine {
            ft,
            faces,
            glyph_cache: HashMap::new(),
            backend: "harfrust+freetype",
        })
    }

    fn resolve_key(&self, family: FontFamily, weight: FontWeight) -> Option<(FontFamily, FontWeight)> {
        let keys = [
            (family, weight),
            (family, FontWeight::Regular),
            (FontFamily::Default, weight),
            (FontFamily::Default, FontWeight::Regular),
        ];
        for k in keys {
            if self.faces.contains_key(&k) {
                return Some(k);
            }
        }
        None
    }

    fn shape(
        &self,
        key: (FontFamily, FontWeight),
        text: &str,
        px: u32,
    ) -> Option<(Vec<(u32, i32, i32)>, i32)> {
        let face = self.faces.get(&key)?;
        let font = harfrust::FontRef::new(&face.bytes).ok()?;
        let data = harfrust::ShaperData::new(&font);
        let shaper = data.shaper(&font).build();
        let mut buffer = harfrust::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        let opts = harfrust::ShapeOptions::new().scale(Some((px * 64) as i32));
        let glyphs = shaper.shape(buffer, opts);
        let mut out = Vec::with_capacity(glyphs.len());
        let mut advance = 0i32;
        for (info, pos) in glyphs.glyph_infos().iter().zip(glyphs.glyph_positions()) {
            out.push((info.glyph_id, pos.x_advance, pos.x_offset));
            advance += pos.x_advance;
        }
        Some((out, advance))
    }

    fn raster_glyph(
        &mut self,
        key: (FontFamily, FontWeight),
        gid: u32,
        px: u32,
    ) -> Option<CachedGlyph> {
        let cache_key = (key.0, key.1, gid, px);
        if let Some(g) = self.glyph_cache.get(&cache_key) {
            return Some(CachedGlyph {
                width: g.width,
                rows: g.rows,
                left: g.left,
                top: g.top,
                buffer: g.buffer.clone(),
            });
        }
        let path = self.faces.get(&key)?.path.clone();
        let face = self.ft.new_face(&path, 0).ok()?;
        face.set_pixel_sizes(0, px).ok()?;
        face.load_glyph(gid, freetype::face::LoadFlag::RENDER).ok()?;
        let glyph = face.glyph();
        let bm = glyph.bitmap();
        let width = bm.width();
        let rows = bm.rows();
        let pitch = bm.pitch();
        let mut buffer = Vec::with_capacity((width * rows).max(0) as usize);
        let raw = bm.buffer();
        for row in 0..rows {
            let start = (row * pitch.abs()) as usize;
            for col in 0..width {
                let idx = if pitch >= 0 {
                    start + col as usize
                } else {
                    start + (width - 1 - col) as usize
                };
                buffer.push(*raw.get(idx).unwrap_or(&0));
            }
        }
        let cached = CachedGlyph {
            width,
            rows,
            left: glyph.bitmap_left(),
            top: glyph.bitmap_top(),
            buffer,
        };
        self.glyph_cache.insert(
            cache_key,
            CachedGlyph {
                width: cached.width,
                rows: cached.rows,
                left: cached.left,
                top: cached.top,
                buffer: cached.buffer.clone(),
            },
        );
        Some(cached)
    }
}

fn face_candidates() -> Vec<(FontFamily, FontWeight, Vec<String>)> {
    let mut roots = Vec::new();
    if let Ok(dir) = std::env::var("THE_MACHINE_FONT_DIR") {
        roots.push(dir);
    }
    roots.push("/etc/the-machine/fonts".into());
    roots.push("/workspace/assets/fonts".into());
    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join("assets/fonts").display().to_string());
    }
    roots.push("/the-machine/fonts".into());
    roots.push("/usr/share/fonts/truetype/macos".into());
    roots.push("/usr/share/fonts/opentype/inter".into());
    roots.push("/usr/share/fonts/truetype/jetbrains-mono".into());

    let specs = [
        (
            FontFamily::Default,
            FontWeight::Regular,
            &["Inter-Regular.ttf", "Inter-Regular.otf"][..],
        ),
        (
            FontFamily::Default,
            FontWeight::Medium,
            &["Inter-Medium.ttf", "Inter-Medium.otf"][..],
        ),
        (
            FontFamily::Default,
            FontWeight::Bold,
            &["Inter-Bold.ttf", "Inter-Bold.otf"][..],
        ),
        (
            FontFamily::Numeric,
            FontWeight::Regular,
            &["JetBrainsMono-Regular.ttf"][..],
        ),
        (
            FontFamily::Numeric,
            FontWeight::Bold,
            &["JetBrainsMono-Bold.ttf"][..],
        ),
    ];
    let mut out = Vec::new();
    for (family, weight, names) in specs {
        let mut paths = Vec::new();
        for root in &roots {
            for name in names {
                paths.push(format!("{root}/{name}"));
            }
        }
        out.push((family, weight, paths));
    }
    out
}

/// Map design-system `font_px` / legacy `font_scale` to pixel size.
pub fn resolve_px(font_px: u32, font_scale: u32) -> u32 {
    if font_px > 0 {
        return font_px.clamp(8, 96);
    }
    match font_scale {
        0 | 1 => 12,
        2 => 14,
        3 => 13,
        4 => 20,
        5 => 26,
        _ => 32,
    }
}

pub fn backend_name() -> &'static str {
    match engine()
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|e| e.backend))
    {
        Some(name) => name,
        None => "bitmap-fallback",
    }
}

pub fn measure_text(
    text: &str,
    font_px: u32,
    weight: FontWeight,
    family: FontFamily,
) -> (u32, u32) {
    let px = font_px.max(8);
    let mut guard = match engine().lock() {
        Ok(g) => g,
        Err(_) => return bitmap_fallback_measure(text, px),
    };
    let Some(eng) = guard.as_mut() else {
        return bitmap_fallback_measure(text, px);
    };
    let Some(key) = eng.resolve_key(family, weight) else {
        return bitmap_fallback_measure(text, px);
    };
    let Some((_, advance)) = eng.shape(key, text, px) else {
        return bitmap_fallback_measure(text, px);
    };
    let width = ((advance + 63) / 64).max(0) as u32;
    let height = (px as f32 * 1.3) as u32;
    (width, height)
}

pub fn draw_text(
    px: &mut PixelBackend,
    x: i32,
    y: i32,
    text: &str,
    rgb: [u8; 3],
    font_px: u32,
    weight: FontWeight,
    family: FontFamily,
) {
    let size = font_px.max(8);
    let mut guard = match engine().lock() {
        Ok(g) => g,
        Err(_) => {
            bitmap_fallback_draw(px, x, y, text, rgb, size);
            return;
        }
    };
    let Some(eng) = guard.as_mut() else {
        bitmap_fallback_draw(px, x, y, text, rgb, size);
        return;
    };
    let Some(key) = eng.resolve_key(family, weight) else {
        bitmap_fallback_draw(px, x, y, text, rgb, size);
        return;
    };
    let Some((glyphs, _)) = eng.shape(key, text, size) else {
        bitmap_fallback_draw(px, x, y, text, rgb, size);
        return;
    };

    let baseline = y + (size as i32 * 4 / 5);
    let mut pen_x = x * 64;
    for (gid, x_advance, x_offset) in glyphs {
        let glyph = match eng.raster_glyph(key, gid, size) {
            Some(g) => g,
            None => {
                pen_x += x_advance;
                continue;
            }
        };
        let dest_x = (pen_x + x_offset) / 64 + glyph.left;
        let dest_y = baseline - glyph.top;
        blit_glyph(px, dest_x, dest_y, &glyph, rgb);
        pen_x += x_advance;
    }
}

fn blit_glyph(px: &mut PixelBackend, x: i32, y: i32, glyph: &CachedGlyph, rgb: [u8; 3]) {
    for row in 0..glyph.rows {
        for col in 0..glyph.width {
            let cover = glyph.buffer[(row * glyph.width + col) as usize] as u16;
            if cover <= 16 {
                continue;
            }
            let rr = ((rgb[0] as u16 * cover) / 255) as u8;
            let gg = ((rgb[1] as u16 * cover) / 255) as u8;
            let bb = ((rgb[2] as u16 * cover) / 255) as u8;
            px.fill_rect(x + col, y + row, 1, 1, [rr, gg, bb]);
        }
    }
}

fn bitmap_fallback_measure(text: &str, px: u32) -> (u32, u32) {
    let scale = (px / 7).max(1);
    (bitmap_font::text_width(text, scale), px)
}

fn bitmap_fallback_draw(
    px: &mut PixelBackend,
    x: i32,
    y: i32,
    text: &str,
    rgb: [u8; 3],
    size: u32,
) {
    bitmap_font::draw_text_scaled(
        px,
        x,
        y,
        text,
        rgb[0],
        rgb[1],
        rgb[2],
        (size / 7).max(1),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harfrust_shapes_and_rasterizes_inter() {
        std::env::set_var("THE_MACHINE_FONT_DIR", "/workspace/assets/fonts");
        std::env::set_var("THE_MACHINE_COMPOSITOR_BACKEND", "memory");
        let (w, h) = measure_text("Welcome back", 20, FontWeight::Bold, FontFamily::Default);
        assert!(w > 40, "width={w}");
        assert!(h >= 20, "height={h}");
        assert_eq!(backend_name(), "harfrust+freetype");

        let mut px = PixelBackend::open();
        px.clear(11, 12, 19);
        draw_text(
            &mut px,
            40,
            40,
            "Welcome back",
            [247, 248, 252],
            20,
            FontWeight::Bold,
            FontFamily::Default,
        );
        px.present();
    }
}
