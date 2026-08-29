//! Framebuffer / DRM / memory pixel backends — paints real RGB pixels for surfaces.

use crate::drm::DrmBackend;

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use tracing::{info, warn};

pub enum BackendKind {
    Drm,
    Framebuffer,
    Memory,
}

pub struct PixelBackend {
    kind: BackendKind,
    width: u32,
    height: u32,
    stride: u32,
    bpp: u32,
    buffer: Vec<u8>,
    fb_mmap: Option<MmapFb>,
    drm: Option<DrmBackend>,
    dump_path: Option<String>,
}

struct MmapFb {
    _file: File,
    ptr: *mut u8,
    len: usize,
}

unsafe impl Send for MmapFb {}

impl PixelBackend {
    pub fn open() -> Self {
        let backend_pref = std::env::var("THE_MACHINE_COMPOSITOR_BACKEND")
            .unwrap_or_else(|_| "auto".into());
        let dump_path = std::env::var("THE_MACHINE_FB_DUMP").ok();

        if matches!(backend_pref.as_str(), "auto" | "drm") {
            if let Some(drm) = DrmBackend::open() {
                let width = drm.width();
                let height = drm.height();
                let len = (width * height * 4) as usize;
                return PixelBackend {
                    kind: BackendKind::Drm,
                    width,
                    height,
                    stride: width * 4,
                    bpp: 32,
                    buffer: vec![0u8; len],
                    fb_mmap: None,
                    drm: Some(drm),
                    dump_path,
                };
            }
            if backend_pref == "drm" {
                warn!("DRM backend requested but unavailable — using memory buffer");
            }
        }

        let fb_path = std::env::var("THE_MACHINE_FB_DEVICE").unwrap_or_else(|_| "/dev/fb0".into());
        if matches!(backend_pref.as_str(), "auto" | "framebuffer") {
            if let Some(fb) = open_framebuffer(&fb_path) {
                info!(
                    "pixel backend: framebuffer {}x{} stride={} bpp={}",
                    fb.width, fb.height, fb.stride, fb.bpp
                );
                let len = (fb.stride * fb.height) as usize;
                return PixelBackend {
                    kind: BackendKind::Framebuffer,
                    width: fb.width,
                    height: fb.height,
                    stride: fb.stride,
                    bpp: fb.bpp,
                    buffer: vec![0u8; len],
                    fb_mmap: Some(fb.mmap),
                    drm: None,
                    dump_path,
                };
            }
        }

        warn!("pixel backend: using 1280x720 memory buffer");
        PixelBackend {
            kind: BackendKind::Memory,
            width: 1280,
            height: 720,
            stride: 1280 * 4,
            bpp: 32,
            buffer: vec![0u8; 1280 * 720 * 4],
            fb_mmap: None,
            drm: None,
            dump_path,
        }
    }

    pub fn backend_name(&self) -> &'static str {
        match self.kind {
            BackendKind::Drm => "drm-kms",
            BackendKind::Framebuffer => "framebuffer",
            BackendKind::Memory => "memory",
        }
    }

    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        let pixel = [b, g, r, 0u8];
        for y in 0..self.height {
            for x in 0..self.width {
                self.put_pixel(x as i32, y as i32, &pixel);
            }
        }
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, r: u8, g: u8, b: u8) {
        let pixel = [b, g, r, 0u8];
        for dy in 0..h {
            for dx in 0..w {
                self.put_pixel(x + dx as i32, y + dy as i32, &pixel);
            }
        }
    }

    pub fn present(&mut self) {
        match self.kind {
            BackendKind::Drm => {
                if let Some(drm) = &mut self.drm {
                    drm.buffer_mut().copy_from_slice(&self.buffer);
                    drm.present();
                }
            }
            BackendKind::Framebuffer => {
                if let Some(fb) = &self.fb_mmap {
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            self.buffer.as_ptr(),
                            fb.ptr,
                            self.buffer.len().min(fb.len),
                        );
                    }
                }
            }
            BackendKind::Memory => {}
        }
        if let Some(path) = &self.dump_path {
            let _ = write_ppm(path, self.width, self.height, &self.buffer);
        }
    }

    fn put_pixel(&mut self, x: i32, y: i32, pixel: &[u8; 4]) {
        if x < 0 || y < 0 {
            return;
        }
        let x = x as u32;
        let y = y as u32;
        if x >= self.width || y >= self.height {
            return;
        }
        let off = (y * self.stride + x * 4) as usize;
        if off + 4 <= self.buffer.len() {
            self.buffer[off..off + 4].copy_from_slice(pixel);
        }
    }
}

struct FbInfo {
    width: u32,
    height: u32,
    stride: u32,
    bpp: u32,
    mmap: MmapFb,
}

fn open_framebuffer(path: &str) -> Option<FbInfo> {
    if !Path::new(path).exists() {
        return None;
    }
    let file = OpenOptions::new().read(true).write(true).open(path).ok()?;
    let (width, height, stride, bpp) = read_fb_info(path)?;
    let len = (stride * height) as usize;
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            file.as_raw_fd(),
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        return None;
    }
    Some(FbInfo {
        width,
        height,
        stride,
        bpp,
        mmap: MmapFb {
            _file: file,
            ptr: ptr as *mut u8,
            len,
        },
    })
}

fn read_fb_info(path: &str) -> Option<(u32, u32, u32, u32)> {
    let sys = format!(
        "/sys/class/graphics/{}/",
        Path::new(path).file_name()?.to_str()?
    );
    let read_u32 = |name: &str| -> Option<u32> {
        std::fs::read_to_string(format!("{}{}", sys, name))
            .ok()?
            .trim()
            .parse()
            .ok()
    };
    let width = read_u32("width").or_else(|| read_virtual_dim(&sys, 0))?;
    let height = read_u32("height").or_else(|| read_virtual_dim(&sys, 1))?;
    let stride = read_u32("stride").unwrap_or(width * 4);
    let bpp = read_u32("bits_per_pixel").unwrap_or(32);
    Some((width, height, stride, bpp))
}

fn read_virtual_dim(sys: &str, idx: usize) -> Option<u32> {
    let s = std::fs::read_to_string(format!("{}virtual_size", sys)).ok()?;
    s.split(',').nth(idx)?.trim().parse().ok()
}

fn write_ppm(path: &str, width: u32, height: u32, buffer: &[u8]) -> std::io::Result<()> {
    let mut f = File::create(path)?;
    writeln!(f, "P6\n{} {}", width, height)?;
    for y in 0..height {
        for x in 0..width {
            let off = (y * width * 4 + x * 4) as usize;
            if off + 3 <= buffer.len() {
                f.write_all(&buffer[off..off + 3])?;
            }
        }
    }
    Ok(())
}

pub type SharedPixel = std::sync::Arc<tokio::sync::Mutex<PixelBackend>>;

pub fn hash_color(id: &str) -> (u8, u8, u8) {
    let h = id.bytes().fold(0u32, |a, b| a.wrapping_mul(31).wrapping_add(b as u32));
    (
        ((h >> 16) & 0xFF) as u8 / 2 + 64,
        ((h >> 8) & 0xFF) as u8 / 2 + 64,
        (h & 0xFF) as u8 / 2 + 64,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_color_is_stable() {
        let (r, g, b) = hash_color("surface.ui.root");
        assert!(r > 0 && g > 0 && b > 0);
        assert_eq!(hash_color("surface.ui.root"), hash_color("surface.ui.root"));
    }

    #[test]
    fn memory_buffer_paints_pixels() {
        std::env::set_var("THE_MACHINE_COMPOSITOR_BACKEND", "memory");
        std::env::set_var("THE_MACHINE_FB_DUMP", "/tmp/compositor-test.ppm");
        let mut px = PixelBackend::open();
        assert_eq!(px.backend_name(), "memory");
        px.clear(0, 0, 0);
        px.fill_rect(10, 10, 50, 30, 255, 0, 0);
        px.present();
        assert!(Path::new("/tmp/compositor-test.ppm").exists());
    }
}
