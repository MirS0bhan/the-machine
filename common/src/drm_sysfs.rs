//! DRM connector discovery via `/sys/class/drm` (no ioctl).

use std::path::Path;

/// Parse `1920x1080` EDID mode lines from sysfs.
pub fn parse_mode_line(line: &str) -> Option<(u32, u32)> {
    let (w, h) = line.trim().split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

/// Preferred resolution from the first connected DRM connector.
pub fn preferred_connector_size() -> Option<(u32, u32)> {
    let drm_root = Path::new("/sys/class/drm");
    let entries = std::fs::read_dir(drm_root).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.contains('-') {
            continue;
        }
        let connector = entry.path();
        let status = std::fs::read_to_string(connector.join("status")).unwrap_or_default();
        if status.trim() != "connected" {
            continue;
        }
        let modes = std::fs::read_to_string(connector.join("modes")).ok()?;
        for line in modes.lines() {
            if let Some((w, h)) = parse_mode_line(line) {
                if w > 0 && h > 0 {
                    return Some((w, h));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mode_line_accepts_width_by_height() {
        assert_eq!(parse_mode_line(" 1920x1080 "), Some((1920, 1080)));
    }
}
