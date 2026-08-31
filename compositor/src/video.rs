//! Video / image decode for `media` surfaces via ffmpeg CLI (when available).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use tracing::warn;

#[derive(Clone, Debug)]
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    /// RGB24 tightly packed.
    pub rgb: Vec<u8>,
    pub source: String,
}

static CACHE: Mutex<Option<DecodedFrame>> = Mutex::new(None);

pub fn ffmpeg_available() -> bool {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn backend_name() -> &'static str {
    if ffmpeg_available() {
        "ffmpeg-cli"
    } else {
        "procedural"
    }
}

/// Decode the first frame of `src` (file path or file:// URL) to RGB24.
pub fn decode_first_frame(src: &str, max_w: u32, max_h: u32) -> Option<DecodedFrame> {
    let path = normalize_src(src)?;
    if !path.exists() {
        warn!("video src missing: {}", path.display());
        return None;
    }
    if let Ok(g) = CACHE.lock() {
        if let Some(frame) = g.as_ref() {
            if frame.source == path.to_string_lossy() {
                return Some(frame.clone());
            }
        }
    }
    if !ffmpeg_available() {
        return None;
    }
    let w = max_w.clamp(64, 1280);
    let h = max_h.clamp(64, 720);
    let scale = format!("scale={w}:{h}:force_original_aspect_ratio=decrease");
    let output = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-i",
            &path.to_string_lossy(),
            "-an",
            "-vf",
            &scale,
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .output()
        .ok()?;
    if !output.status.success() || output.stdout.is_empty() {
        warn!(
            "ffmpeg decode failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        return None;
    }
    // Infer dimensions from probe; fall back to requested size if stdout length matches.
    let (fw, fh) = probe_size(&path).unwrap_or((w, h));
    let (fw, fh) = fit_within(fw, fh, w, h);
    let expected = (fw * fh * 3) as usize;
    let rgb = if output.stdout.len() >= expected {
        output.stdout[..expected].to_vec()
    } else {
        // ffmpeg may have chosen slightly different size — use closest rectangle.
        let pixels = output.stdout.len() / 3;
        let fw = ((pixels as f64).sqrt() as u32).max(1);
        let fh = (pixels as u32 / fw).max(1);
        let take = (fw * fh * 3) as usize;
        output.stdout[..take.min(output.stdout.len())].to_vec()
    };
    let actual_pixels = rgb.len() / 3;
    let fw = if actual_pixels > 0 {
        // Prefer probed fitted width when buffer matches.
        if (fw * fh) as usize == actual_pixels {
            fw
        } else {
            ((actual_pixels as f64).sqrt() as u32).max(1)
        }
    } else {
        return None;
    };
    let fh = (actual_pixels as u32 / fw).max(1);
    let frame = DecodedFrame {
        width: fw,
        height: fh,
        rgb,
        source: path.to_string_lossy().into_owned(),
    };
    if let Ok(mut g) = CACHE.lock() {
        *g = Some(frame.clone());
    }
    Some(frame)
}

fn normalize_src(src: &str) -> Option<PathBuf> {
    let s = src.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(rest) = s.strip_prefix("file://") {
        return Some(PathBuf::from(rest));
    }
    Some(PathBuf::from(s))
}

fn probe_size(path: &Path) -> Option<(u32, u32)> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0:s=x",
            &path.to_string_lossy(),
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let mut parts = text.trim().split('x');
    let w = parts.next()?.parse().ok()?;
    let h = parts.next()?.parse().ok()?;
    Some((w, h))
}

fn fit_within(w: u32, h: u32, max_w: u32, max_h: u32) -> (u32, u32) {
    if w == 0 || h == 0 {
        return (max_w, max_h);
    }
    let sx = max_w as f64 / w as f64;
    let sy = max_h as f64 / h as f64;
    let s = sx.min(sy).min(1.0);
    (
        ((w as f64) * s).round().max(1.0) as u32,
        ((h as f64) * s).round().max(1.0) as u32,
    )
}

/// Blit an RGB24 frame into BGRA framebuffer coordinates.
pub fn blit_rgb_frame(
    px: &mut crate::pixel::PixelBackend,
    x: i32,
    y: i32,
    frame: &DecodedFrame,
) {
    let mut bgra = Vec::with_capacity(frame.rgb.len() / 3 * 4);
    for chunk in frame.rgb.chunks_exact(3) {
        bgra.push(chunk[2]); // B
        bgra.push(chunk[1]); // G
        bgra.push(chunk[0]); // R
        bgra.push(0);
    }
    px.blit_bgra(x, y, frame.width, frame.height, frame.width * 4, &bgra);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_file_url() {
        let p = normalize_src("file:///tmp/a.mp4").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/a.mp4"));
    }

    #[test]
    fn fit_does_not_upscale() {
        let (w, h) = fit_within(100, 50, 200, 200);
        assert_eq!((w, h), (100, 50));
    }
}
