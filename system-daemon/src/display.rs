//! Display mode queries and mutations via sysfs reads + DRM/KMS set when available.

use common::DisplayMode;
use std::fs::{self, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

const DRM_IOCTL_MODE_GETRESOURCES: libc::c_ulong = 0xc04064a0;
const DRM_IOCTL_MODE_GETCONNECTOR: libc::c_ulong = 0xc06464a7;
const DRM_IOCTL_MODE_SETCRTC: libc::c_ulong = 0xc06864a2;

#[repr(C)]
struct DrmModeRes {
    fb_id_ptr: u64,
    crtc_id_ptr: u64,
    connector_id_ptr: u64,
    encoder_id_ptr: u64,
    count_fbs: u32,
    count_crtcs: u32,
    count_connectors: u32,
    count_encoders: u32,
    min_width: u32,
    max_width: u32,
    min_height: u32,
    max_height: u32,
}

#[repr(C)]
struct DrmModeConnector {
    encoders_ptr: u64,
    modes_ptr: u64,
    props_ptr: u64,
    prop_values_ptr: u64,
    count_modes: u32,
    count_props: u32,
    count_encoders: u32,
    encoder_id: u32,
    connector_id: u32,
    connector_type: u32,
    connector_type_id: u32,
    connection: u32,
    mm_width: u32,
    mm_height: u32,
    subpixel: u32,
    pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct DrmModeModeInfo {
    clock: u32,
    hdisplay: u16,
    hsync_start: u16,
    hsync_end: u16,
    htotal: u16,
    hskew: u16,
    vdisplay: u16,
    vsync_start: u16,
    vsync_end: u16,
    vtotal: u16,
    vscan: u16,
    vrefresh: u32,
    flags: u32,
    type_: u32,
    name: [u8; 32],
}

#[repr(C)]
struct DrmModeCrtc {
    set_connectors_ptr: u64,
    count_connectors: u32,
    crtc_id: u32,
    fb_id: u32,
    x: u32,
    y: u32,
    gamma_size: u32,
    mode_valid: u32,
    mode: DrmModeModeInfo,
}

fn drm_device_path() -> PathBuf {
    std::env::var("THE_MACHINE_DRM_DEVICE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/dev/dri/card0"))
}

pub fn get_display_modes() -> Vec<DisplayMode> {
    if let Some(modes) = read_modes_from_sysfs() {
        if !modes.is_empty() {
            return modes;
        }
    }
    if let Some(modes) = read_modes_from_drm() {
        if !modes.is_empty() {
            return modes;
        }
    }
    fallback_modes()
}

pub fn set_display_mode(width: u32, height: u32, refresh: f32) -> Result<(), String> {
    if width == 0 || height == 0 {
        return Err("width and height must be non-zero".into());
    }
    let path = drm_device_path();
    if !path.exists() {
        return Err(format!(
            "display mode change requires DRM/KMS; {} is not present on this host",
            path.display()
        ));
    }
    let card = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    set_crtc_mode(card.as_raw_fd(), width, height, refresh)
        .map_err(|e| format!("DRM set mode failed on {}: {e}", path.display()))
}

fn fallback_modes() -> Vec<DisplayMode> {
    vec![DisplayMode {
        width: 1920,
        height: 1080,
        refresh: 60.0,
        current: true,
    }]
}

fn read_modes_from_sysfs() -> Option<Vec<DisplayMode>> {
    let drm_root = Path::new("/sys/class/drm");
    let entries = fs::read_dir(drm_root).ok()?;
    let mut modes = Vec::new();
    let mut current = None;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.contains('-') || name.ends_with("-DP-") {
            continue;
        }
        let connector_path = entry.path();
        let status = fs::read_to_string(connector_path.join("status")).unwrap_or_default();
        let connected = status.trim() == "connected";
        let enabled = fs::read_to_string(connector_path.join("enabled"))
            .map(|s| s.trim() == "enabled")
            .unwrap_or(false);
        let modes_path = connector_path.join("modes");
        let body = fs::read_to_string(&modes_path).ok()?;
        for line in body.lines() {
            if let Some((w, h)) = parse_mode_line(line) {
                let mode = DisplayMode {
                    width: w,
                    height: h,
                    refresh: 60.0,
                    current: connected && enabled && current.is_none(),
                };
                if mode.current {
                    current = Some(modes.len());
                }
                modes.push(mode);
            }
        }
    }

    if modes.is_empty() {
        None
    } else {
        if current.is_none() {
            modes[0].current = true;
        }
        Some(modes)
    }
}

fn parse_mode_line(line: &str) -> Option<(u32, u32)> {
    let (w, h) = line.trim().split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

fn read_modes_from_drm() -> Option<Vec<DisplayMode>> {
    let path = drm_device_path();
    if !path.exists() {
        return None;
    }
    let card = OpenOptions::new().read(true).write(true).open(&path).ok()?;
    let fd = card.as_raw_fd();
    let connector_id = first_connected_connector(fd)?;
    let mut conn = DrmModeConnector {
        encoders_ptr: 0,
        modes_ptr: 0,
        props_ptr: 0,
        prop_values_ptr: 0,
        count_modes: 0,
        count_props: 0,
        count_encoders: 0,
        encoder_id: 0,
        connector_id,
        connector_type: 0,
        connector_type_id: 0,
        connection: 0,
        mm_width: 0,
        mm_height: 0,
        subpixel: 0,
        pad: 0,
    };
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR as _, &mut conn) } != 0 {
        return None;
    }
    if conn.count_modes == 0 {
        return None;
    }
    let mut mode_infos = vec![
        DrmModeModeInfo {
            clock: 0,
            hdisplay: 0,
            hsync_start: 0,
            hsync_end: 0,
            htotal: 0,
            hskew: 0,
            vdisplay: 0,
            vsync_start: 0,
            vsync_end: 0,
            vtotal: 0,
            vscan: 0,
            vrefresh: 0,
            flags: 0,
            type_: 0,
            name: [0; 32],
        };
        conn.count_modes as usize
    ];
    conn.modes_ptr = mode_infos.as_mut_ptr() as u64;
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR as _, &mut conn) } != 0 {
        return None;
    }
    Some(
        mode_infos
            .into_iter()
            .enumerate()
            .map(|(idx, mode)| DisplayMode {
                width: mode.hdisplay as u32,
                height: mode.vdisplay as u32,
                refresh: mode.vrefresh as f32,
                current: idx == 0,
            })
            .collect(),
    )
}

fn first_connected_connector(fd: i32) -> Option<u32> {
    let mut res = DrmModeRes {
        fb_id_ptr: 0,
        crtc_id_ptr: 0,
        connector_id_ptr: 0,
        encoder_id_ptr: 0,
        count_fbs: 0,
        count_crtcs: 0,
        count_connectors: 0,
        count_encoders: 0,
        min_width: 0,
        max_width: 0,
        min_height: 0,
        max_height: 0,
    };
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES as _, &mut res) } != 0 {
        return None;
    }
    if res.count_connectors == 0 {
        return None;
    }
    let mut connector_ids = vec![0u32; res.count_connectors as usize];
    res.connector_id_ptr = connector_ids.as_mut_ptr() as u64;
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES as _, &mut res) } != 0 {
        return None;
    }
    for connector_id in &connector_ids {
        let mut conn = DrmModeConnector {
            encoders_ptr: 0,
            modes_ptr: 0,
            props_ptr: 0,
            prop_values_ptr: 0,
            count_modes: 0,
            count_props: 0,
            count_encoders: 0,
            encoder_id: 0,
            connector_id: *connector_id,
            connector_type: 0,
            connector_type_id: 0,
            connection: 0,
            mm_width: 0,
            mm_height: 0,
            subpixel: 0,
            pad: 0,
        };
        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR as _, &mut conn) } != 0 {
            continue;
        }
        // DRM_MODE_CONNECTED = 1
        if conn.connection == 1 && conn.count_modes > 0 {
            return Some(*connector_id);
        }
    }
    connector_ids.first().copied()
}

fn set_crtc_mode(fd: i32, width: u32, height: u32, refresh: f32) -> Result<(), String> {
    let mut res = DrmModeRes {
        fb_id_ptr: 0,
        crtc_id_ptr: 0,
        connector_id_ptr: 0,
        encoder_id_ptr: 0,
        count_fbs: 0,
        count_crtcs: 0,
        count_connectors: 0,
        count_encoders: 0,
        min_width: 0,
        max_width: 0,
        min_height: 0,
        max_height: 0,
    };
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES as _, &mut res) } != 0 {
        return Err("MODE_GETRESOURCES failed".into());
    }
    if res.count_crtcs == 0 || res.count_connectors == 0 {
        return Err("no CRTCs or connectors available".into());
    }
    let mut crtc_ids = vec![0u32; res.count_crtcs as usize];
    let mut connector_ids = vec![0u32; res.count_connectors as usize];
    res.crtc_id_ptr = crtc_ids.as_mut_ptr() as u64;
    res.connector_id_ptr = connector_ids.as_mut_ptr() as u64;
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES as _, &mut res) } != 0 {
        return Err("MODE_GETRESOURCES (ids) failed".into());
    }

    let connector_id = first_connected_connector(fd)
        .ok_or_else(|| "no connected DRM connector with modes".to_string())?;
    let mut conn = DrmModeConnector {
        encoders_ptr: 0,
        modes_ptr: 0,
        props_ptr: 0,
        prop_values_ptr: 0,
        count_modes: 0,
        count_props: 0,
        count_encoders: 0,
        encoder_id: 0,
        connector_id,
        connector_type: 0,
        connector_type_id: 0,
        connection: 0,
        mm_width: 0,
        mm_height: 0,
        subpixel: 0,
        pad: 0,
    };
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR as _, &mut conn) } != 0 {
        return Err("MODE_GETCONNECTOR failed".into());
    }
    if conn.count_modes == 0 {
        return Err("connector has no modes".into());
    }
    let mut mode_infos = vec![
        DrmModeModeInfo {
            clock: 0,
            hdisplay: 0,
            hsync_start: 0,
            hsync_end: 0,
            htotal: 0,
            hskew: 0,
            vdisplay: 0,
            vsync_start: 0,
            vsync_end: 0,
            vtotal: 0,
            vscan: 0,
            vrefresh: 0,
            flags: 0,
            type_: 0,
            name: [0; 32],
        };
        conn.count_modes as usize
    ];
    conn.modes_ptr = mode_infos.as_mut_ptr() as u64;
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETCONNECTOR as _, &mut conn) } != 0 {
        return Err("MODE_GETCONNECTOR (modes) failed".into());
    }

    let selected = mode_infos
        .iter()
        .find(|mode| {
            mode.hdisplay as u32 == width
                && mode.vdisplay as u32 == height
                && (refresh <= 0.0 || (mode.vrefresh as f32 - refresh).abs() < 1.0)
        })
        .or_else(|| {
            mode_infos
                .iter()
                .find(|mode| mode.hdisplay as u32 == width && mode.vdisplay as u32 == height)
        })
        .ok_or_else(|| format!("no DRM mode matching {width}x{height}@{refresh}"))?;

    let connector = connector_id;
    let mut crtc = DrmModeCrtc {
        set_connectors_ptr: &connector as *const u32 as u64,
        count_connectors: 1,
        crtc_id: crtc_ids[0],
        fb_id: 0,
        x: 0,
        y: 0,
        gamma_size: 0,
        mode_valid: 1,
        mode: *selected,
    };
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_SETCRTC as _, &mut crtc) } != 0 {
        return Err("MODE_SETCRTC failed (no framebuffer bound)".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mode_line_accepts_width_by_height() {
        assert_eq!(parse_mode_line("1920x1080"), Some((1920, 1080)));
        assert_eq!(parse_mode_line(" 1280x720 "), Some((1280, 720)));
        assert!(parse_mode_line("invalid").is_none());
    }

    #[test]
    fn set_display_mode_without_drm_device_is_unavailable() {
        let missing = PathBuf::from("/tmp/the-machine-no-drm-card0");
        std::env::set_var("THE_MACHINE_DRM_DEVICE", missing.to_string_lossy().as_ref());
        let err = set_display_mode(1920, 1080, 60.0).unwrap_err();
        assert!(err.contains("requires DRM/KMS"));
        std::env::remove_var("THE_MACHINE_DRM_DEVICE");
    }

    #[test]
    fn fallback_modes_returns_single_hd_default() {
        let modes = fallback_modes();
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0].width, 1920);
        assert_eq!(modes[0].height, 1080);
        assert!(modes[0].current);
    }

    #[test]
    fn get_display_modes_returns_non_empty_on_host() {
        let modes = get_display_modes();
        assert!(!modes.is_empty());
        assert!(modes.iter().any(|m| m.width > 0 && m.height > 0));
    }
}
