//! Minimal xdg-shell (`xdg_wm_base`) for G17 — third-party Wayland toplevels.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use tracing::info;
use wayland_protocols::xdg::shell::server::{
    xdg_positioner::{self, XdgPositioner},
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::{self, XdgToplevel},
    xdg_wm_base::{self, XdgWmBase},
};
use wayland_server::{protocol::wl_surface::WlSurface, Display, DisplayHandle, GlobalDispatch};

use crate::wl_session::CompositorState;

static CONFIGURE_SERIAL: AtomicU32 = AtomicU32::new(1);
static XDG_TOPLEVEL_COUNT: AtomicU32 = AtomicU32::new(0);

pub fn register_xdg_shell_global(display: &Display<CompositorState>) {
    // v5: configure/ack without wm_capabilities (v6+).
    display
        .handle()
        .create_global::<CompositorState, XdgWmBase, _>(5, ());
    info!("registered xdg_wm_base (xdg-shell v5)");
}

pub fn toplevel_count() -> u32 {
    XDG_TOPLEVEL_COUNT.load(Ordering::Relaxed)
}

struct XdgSurfaceData {
    wl_surface: WlSurface,
    last_ack: Mutex<Option<u32>>,
}

struct XdgToplevelData {
    xdg_surface: XdgSurface,
    title: Mutex<String>,
    app_id: Mutex<String>,
}

impl GlobalDispatch<XdgWmBase, ()> for CompositorState {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &wayland_server::Client,
        resource: wayland_server::New<XdgWmBase>,
        _global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl wayland_server::Dispatch<XdgWmBase, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &XdgWmBase,
        request: xdg_wm_base::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            xdg_wm_base::Request::CreatePositioner { id } => {
                let _ = data_init.init(id, ());
            }
            xdg_wm_base::Request::GetXdgSurface { id, surface } => {
                let _ = data_init.init(
                    id,
                    XdgSurfaceData {
                        wl_surface: surface,
                        last_ack: Mutex::new(None),
                    },
                );
            }
            xdg_wm_base::Request::Pong { .. } | xdg_wm_base::Request::Destroy => {}
            _ => {}
        }
    }
}

impl wayland_server::Dispatch<XdgSurface, XdgSurfaceData> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        xdg_surface: &XdgSurface,
        request: xdg_surface::Request,
        data: &XdgSurfaceData,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            xdg_surface::Request::GetToplevel { id } => {
                let toplevel = data_init.init(
                    id,
                    XdgToplevelData {
                        xdg_surface: xdg_surface.clone(),
                        title: Mutex::new(String::new()),
                        app_id: Mutex::new(String::new()),
                    },
                );
                XDG_TOPLEVEL_COUNT.fetch_add(1, Ordering::Relaxed);
                // Initial configure — width/height 0 lets the client choose.
                toplevel.configure(0, 0, vec![]);
                let serial = CONFIGURE_SERIAL.fetch_add(1, Ordering::Relaxed);
                xdg_surface.configure(serial);
                let _ = &data.wl_surface;
            }
            xdg_surface::Request::AckConfigure { serial } => {
                *data.last_ack.lock().unwrap() = Some(serial);
            }
            xdg_surface::Request::SetWindowGeometry { .. } | xdg_surface::Request::Destroy => {}
            _ => {}
        }
    }
}

impl wayland_server::Dispatch<XdgToplevel, XdgToplevelData> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &XdgToplevel,
        request: xdg_toplevel::Request,
        data: &XdgToplevelData,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            xdg_toplevel::Request::SetTitle { title } => {
                *data.title.lock().unwrap() = title;
            }
            xdg_toplevel::Request::SetAppId { app_id } => {
                *data.app_id.lock().unwrap() = app_id;
            }
            xdg_toplevel::Request::Destroy => {
                XDG_TOPLEVEL_COUNT.fetch_sub(1, Ordering::Relaxed);
            }
            xdg_toplevel::Request::SetMinSize { .. }
            | xdg_toplevel::Request::SetMaxSize { .. }
            | xdg_toplevel::Request::SetMaximized
            | xdg_toplevel::Request::UnsetMaximized
            | xdg_toplevel::Request::SetFullscreen { .. }
            | xdg_toplevel::Request::UnsetFullscreen
            | xdg_toplevel::Request::SetMinimized => {}
            _ => {}
        }
        let _ = &data.xdg_surface;
    }
}

impl wayland_server::Dispatch<XdgPositioner, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &XdgPositioner,
        _request: xdg_positioner::Request,
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
    fn configure_serial_advances() {
        let a = CONFIGURE_SERIAL.fetch_add(1, Ordering::Relaxed);
        let b = CONFIGURE_SERIAL.fetch_add(1, Ordering::Relaxed);
        assert!(b > a);
    }
}
