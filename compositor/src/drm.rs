//! DRM/KMS dumb-buffer backend — real GPU scanout when `/dev/dri/card0` is available.

use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use tracing::{info, warn};

const DRM_IOCTL_MODE_CREATE_DUMB: libc::c_ulong = 0xc02064b2;
const DRM_IOCTL_MODE_MAP_DUMB: libc::c_ulong = 0xc01064b3;
const DRM_IOCTL_MODE_DESTROY_DUMB: libc::c_ulong = 0xc01064b4;
const DRM_IOCTL_MODE_SETCRTC: libc::c_ulong = 0xc06864a2;
const DRM_IOCTL_MODE_GETRESOURCES: libc::c_ulong = 0xc04064a0;

#[repr(C)]
struct DrmModeCreateDumb {
    height: u32,
    width: u32,
    bpp: u32,
    flags: u32,
    handle: u32,
    pitch: u32,
    size: u64,
}

#[repr(C)]
struct DrmModeMapDumb {
    handle: u32,
    pad: u32,
    offset: u64,
}

#[repr(C)]
struct DrmModeDestroyDumb {
    handle: u32,
}

#[repr(C)]
struct DrmModeFbCmd2 {
    fb_id: u32,
    width: u32,
    height: u32,
    pixel_format: u32,
    flags: u32,
    handles: [u32; 4],
    pitches: [u32; 4],
    offsets: [u32; 4],
    modifier: [u64; 4],
    modifier_count: u32,
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

pub struct DrmBackend {
    _card: File,
    width: u32,
    height: u32,
    _stride: u32,
    buffer: Vec<u8>,
    dumb_handle: u32,
    _fb_id: u32,
    mmap: DrmMmap,
}

struct DrmMmap {
    ptr: *mut u8,
    len: usize,
}

unsafe impl Send for DrmMmap {}

impl DrmBackend {
    pub fn open() -> Option<Self> {
        let path = common::paths::drm_device_path();
        if !Path::new(&path).exists() {
            return None;
        }
        let card = OpenOptions::new().read(true).write(true).open(&path).ok()?;
        let fd = card.as_raw_fd();
        let (width, height) = preferred_drm_size().unwrap_or((1280u32, 720u32));
        let mut create = DrmModeCreateDumb {
            height,
            width,
            bpp: 32,
            flags: 0,
            handle: 0,
            pitch: 0,
            size: 0,
        };
        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_CREATE_DUMB as _, &mut create) } != 0 {
            warn!("DRM: MODE_CREATE_DUMB failed on {}", path);
            return None;
        }
        let mut map = DrmModeMapDumb {
            handle: create.handle,
            pad: 0,
            offset: 0,
        };
        if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_MAP_DUMB as _, &mut map) } != 0 {
            let mut destroy = DrmModeDestroyDumb {
                handle: create.handle,
            };
            let _ = unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_DESTROY_DUMB as _, &mut destroy) };
            return None;
        }
        let len = create.size as usize;
        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                fd,
                map.offset as libc::off_t,
            )
        };
        if ptr == libc::MAP_FAILED {
            let mut destroy = DrmModeDestroyDumb {
                handle: create.handle,
            };
            let _ = unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_DESTROY_DUMB as _, &mut destroy) };
            return None;
        }
        let mut fb_id = 0u32;
        let mut fb_cmd = DrmModeFbCmd2 {
            fb_id: 0,
            width,
            height,
            pixel_format: fourcc(b"XR24"),
            flags: 0,
            handles: [create.handle, 0, 0, 0],
            pitches: [create.pitch, 0, 0, 0],
            offsets: [0, 0, 0, 0],
            modifier: [0, 0, 0, 0],
            modifier_count: 0,
        };
        // DRM_IOCTL_MODE_ADDFB2 = 0xc0b064b8
        if unsafe { libc::ioctl(fd, 0xc0b064b8u64 as libc::c_ulong, &mut fb_cmd) } == 0 {
            fb_id = fb_cmd.fb_id;
        }
        let _ = set_crtc(fd, fb_id, width, height);
        info!(
            "DRM/KMS backend: {} {}x{} pitch={} (dumb buffer)",
            path, width, height, create.pitch
        );
        Some(DrmBackend {
            _card: card,
            width,
            height,
            _stride: create.pitch,
            buffer: vec![0u8; len],
            dumb_handle: create.handle,
            _fb_id: fb_id,
            mmap: DrmMmap {
                ptr: ptr as *mut u8,
                len,
            },
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn present(&mut self) {
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.buffer.as_ptr(),
                self.mmap.ptr,
                self.buffer.len().min(self.mmap.len),
            );
        }
    }

    pub fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
}

impl Drop for DrmBackend {
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.mmap.ptr as *mut _, self.mmap.len);
        }
        let fd = self._card.as_raw_fd();
        let mut destroy = DrmModeDestroyDumb {
            handle: self.dumb_handle,
        };
        let _ = unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_DESTROY_DUMB as _, &mut destroy) };
    }
}

fn fourcc(code: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*code)
}

fn set_crtc(fd: i32, fb_id: u32, width: u32, height: u32) -> bool {
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
        return false;
    }
    if res.count_crtcs == 0 || res.count_connectors == 0 {
        return false;
    }
    let mut crtc_ids = vec![0u32; res.count_crtcs as usize];
    let mut connector_ids = vec![0u32; res.count_connectors as usize];
    res.crtc_id_ptr = crtc_ids.as_mut_ptr() as u64;
    res.connector_id_ptr = connector_ids.as_mut_ptr() as u64;
    if unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_GETRESOURCES as _, &mut res) } != 0 {
        return false;
    }
    let mode = DrmModeModeInfo {
        clock: 0,
        hdisplay: width as u16,
        hsync_start: 0,
        hsync_end: 0,
        htotal: 0,
        hskew: 0,
        vdisplay: height as u16,
        vsync_start: 0,
        vsync_end: 0,
        vtotal: 0,
        vscan: 0,
        vrefresh: 60,
        flags: 0,
        type_: 0,
        name: [0; 32],
    };
    let connector = connector_ids[0];
    let mut crtc = DrmModeCrtc {
        set_connectors_ptr: &connector as *const u32 as u64,
        count_connectors: 1,
        crtc_id: crtc_ids[0],
        fb_id,
        x: 0,
        y: 0,
        gamma_size: 0,
        mode_valid: 1,
        mode,
    };
    unsafe { libc::ioctl(fd, DRM_IOCTL_MODE_SETCRTC as _, &mut crtc) == 0 }
}

pub fn backend_available() -> bool {
    Path::new("/dev/dri/card0").exists()
}

pub fn preferred_drm_size() -> Option<(u32, u32)> {
    common::preferred_connector_size()
}
