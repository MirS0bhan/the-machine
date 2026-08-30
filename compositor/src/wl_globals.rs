//! Wayland protocol globals (G17) — `wl_compositor`, `wl_output`, `wl_seat`.
//!
//! Uses `wayland-server` directly (no wlroots C dependency). Clients can bind
//! core globals; `wl_surface` commit blits SHM buffers into the pixel backend.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tracing::{info, warn};
use wayland_server::{
    protocol::wl_compositor::{self, WlCompositor},
    protocol::wl_keyboard::{self, WlKeyboard},
    protocol::wl_output::{self, Subpixel, Transform, WlOutput},
    protocol::wl_pointer::{self, WlPointer},
    protocol::wl_seat::{self, WlSeat},
    protocol::wl_surface::{self, WlSurface},
    protocol::wl_touch::{self, WlTouch},
    Display, DisplayHandle, GlobalDispatch, Resource,
};

use crate::wl_session::CompositorState;
use crate::wl_shm::BufferData;

#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub refresh_mhz: i32,
}

impl Default for OutputInfo {
    fn default() -> Self {
        Self {
            name: "the-machine-0".into(),
            width: crate::env::DEFAULT_WIDTH as i32,
            height: crate::env::DEFAULT_HEIGHT as i32,
            refresh_mhz: 60_000,
        }
    }
}

struct OutputGlobal {
    info: OutputInfo,
}

struct SeatGlobal;

struct SurfaceData {
    inner: Mutex<SurfaceInner>,
}

struct SurfaceInner {
    attached: Option<BufferData>,
    x: i32,
    y: i32,
}

/// Background display dispatch + registered globals.
pub struct WlGlobals {
    pub output: OutputInfo,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl WlGlobals {
    pub fn spawn(
        display: Display<CompositorState>,
        output: OutputInfo,
        state: Arc<Mutex<CompositorState>>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("wl-display".into())
            .spawn(move || {
                run_display_loop(display, state, stop_flag);
            })
            .ok();
        if thread.is_none() {
            warn!("failed to spawn wl_display dispatch thread");
        }
        Self {
            output,
            stop,
            thread,
        }
    }

    pub fn status(&self) -> serde_json::Value {
        serde_json::json!({
            "globals": ["wl_compositor", "wl_output", "wl_seat", "wl_shm"],
            "output": {
                "name": self.output.name,
                "width": self.output.width,
                "height": self.output.height,
                "refresh_mhz": self.output.refresh_mhz,
            },
            "wlroots": false,
        })
    }
}

impl Drop for WlGlobals {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

pub fn register_globals(
    display: &Display<CompositorState>,
    output: OutputInfo,
) -> Result<(), String> {
    let handle = display.handle();
    handle.create_global::<CompositorState, WlCompositor, _>(4, ());
    handle.create_global::<CompositorState, WlOutput, _>(
        3,
        OutputGlobal {
            info: output.clone(),
        },
    );
    handle.create_global::<CompositorState, WlSeat, _>(7, SeatGlobal);
    info!(
        "registered wl_compositor, wl_output ({}x{}), wl_seat, wl_shm",
        output.width, output.height
    );
    Ok(())
}

fn run_display_loop(
    mut display: Display<CompositorState>,
    state: Arc<Mutex<CompositorState>>,
    stop: Arc<AtomicBool>,
) {
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let mut st = match state.lock() {
            Ok(g) => g,
            Err(_) => break,
        };
        if let Err(e) = display.dispatch_clients(&mut *st) {
            warn!("wl_display dispatch error: {e}");
            break;
        }
        drop(st);
        if let Err(e) = display.flush_clients() {
            warn!("wl_display flush error: {e}");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(
            crate::env::DEFAULT_FRAME_MS,
        ));
    }
}

impl GlobalDispatch<WlCompositor, ()> for CompositorState {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &wayland_server::Client,
        resource: wayland_server::New<WlCompositor>,
        _global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl wayland_server::Dispatch<WlCompositor, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &WlCompositor,
        request: wl_compositor::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        if let wl_compositor::Request::CreateSurface { id } = request {
            let _surface = data_init.init(
                id,
                SurfaceData {
                    inner: Mutex::new(SurfaceInner {
                        attached: None,
                        x: 0,
                        y: 0,
                    }),
                },
            );
        }
    }
}

impl GlobalDispatch<WlOutput, OutputGlobal> for CompositorState {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &wayland_server::Client,
        resource: wayland_server::New<WlOutput>,
        global_data: &OutputGlobal,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let output = data_init.init(resource, global_data.info.clone());
        output.done();
        output.name(global_data.info.name.clone());
        output.geometry(
            0,
            0,
            global_data.info.width,
            global_data.info.height,
            Subpixel::Unknown,
            String::new(),
            global_data.info.name.clone(),
            Transform::Normal,
        );
        output.mode(
            wl_output::Mode::Current | wl_output::Mode::Preferred,
            global_data.info.width,
            global_data.info.height,
            global_data.info.refresh_mhz,
        );
        output.done();
    }
}

impl wayland_server::Dispatch<WlOutput, OutputInfo> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &WlOutput,
        _request: wl_output::Request,
        _data: &OutputInfo,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
    }
}

impl GlobalDispatch<WlSeat, SeatGlobal> for CompositorState {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &wayland_server::Client,
        resource: wayland_server::New<WlSeat>,
        _global_data: &SeatGlobal,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let seat = data_init.init(resource, ());
        seat.capabilities(
            wl_seat::Capability::Pointer
                | wl_seat::Capability::Keyboard
                | wl_seat::Capability::Touch,
        );
    }
}

impl wayland_server::Dispatch<WlSeat, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &WlSeat,
        request: wl_seat::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            wl_seat::Request::GetPointer { id } => {
                let _ = data_init.init(id, ());
            }
            wl_seat::Request::GetKeyboard { id } => {
                let _ = data_init.init(id, ());
            }
            wl_seat::Request::GetTouch { id } => {
                let _ = data_init.init(id, ());
            }
            _ => {}
        }
    }
}

impl wayland_server::Dispatch<WlSurface, SurfaceData> for CompositorState {
    fn request(
        state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &WlSurface,
        request: wl_surface::Request,
        data: &SurfaceData,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        let mut inner = data.inner.lock().unwrap();
        match request {
            wl_surface::Request::Attach { buffer, x, y } => {
                inner.x = x;
                inner.y = y;
                inner.attached = buffer
                    .as_ref()
                    .and_then(|b| b.data::<BufferData>().cloned());
            }
            wl_surface::Request::Commit => {
                if let (Some(buf), Some(pixels)) = (&inner.attached, &state.pixels) {
                    let offset = buf.offset.max(0) as usize;
                    let need = (buf.stride * buf.height) as usize;
                    let mem = if offset + need <= buf.pool.len() {
                        &buf.pool[offset..offset + need]
                    } else if offset < buf.pool.len() {
                        &buf.pool[offset..]
                    } else {
                        return;
                    };
                    let mut px = pixels.blocking_lock();
                    px.blit_bgra(
                        inner.x,
                        inner.y,
                        buf.width as u32,
                        buf.height as u32,
                        buf.stride as u32,
                        mem,
                    );
                    px.present();
                }
            }
            wl_surface::Request::Damage { .. } => {}
            wl_surface::Request::Frame { .. } => {}
            wl_surface::Request::SetOpaqueRegion { .. } => {}
            wl_surface::Request::SetInputRegion { .. } => {}
            wl_surface::Request::Destroy => {}
            _ => {}
        }
    }
}

impl wayland_server::Dispatch<WlPointer, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &WlPointer,
        _request: wl_pointer::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
    }
}

impl wayland_server::Dispatch<WlKeyboard, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &WlKeyboard,
        _request: wl_keyboard::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
    }
}

impl wayland_server::Dispatch<WlTouch, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &WlTouch,
        _request: wl_touch::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_info_defaults_to_hd() {
        let o = OutputInfo::default();
        assert_eq!(o.width, crate::env::DEFAULT_WIDTH as i32);
        assert_eq!(o.height, crate::env::DEFAULT_HEIGHT as i32);
    }
}
