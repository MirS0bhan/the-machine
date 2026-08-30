//! `wl_shm` pools/buffers — shared memory for client surface pixels (G17).

use std::os::unix::io::AsRawFd;
use std::sync::Arc;

use tracing::warn;
use wayland_server::{
    protocol::wl_buffer::{self, WlBuffer},
    protocol::wl_shm::{self, WlShm},
    protocol::wl_shm_pool::{self, WlShmPool},
    DisplayHandle, GlobalDispatch,
};

use crate::wl_session::CompositorState;

#[derive(Clone)]
pub(crate) struct ShmPoolData {
    pub memory: Arc<[u8]>,
}

#[derive(Clone)]
pub(crate) struct BufferData {
    pub pool: Arc<[u8]>,
    pub offset: i32,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
}

pub fn register_shm_global(display: &wayland_server::Display<CompositorState>) {
    display
        .handle()
        .create_global::<CompositorState, WlShm, _>(1, ());
}

impl GlobalDispatch<WlShm, ()> for CompositorState {
    fn bind(
        _state: &mut Self,
        _handle: &DisplayHandle,
        _client: &wayland_server::Client,
        resource: wayland_server::New<WlShm>,
        _global_data: &(),
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        data_init.init(resource, ());
    }
}

impl wayland_server::Dispatch<WlShm, ()> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &WlShm,
        request: wl_shm::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        if let wl_shm::Request::CreatePool { id, fd, size } = request {
            let fd = fd.as_raw_fd();
            let memory = match map_shm_fd(fd, size as usize) {
                Ok(m) => m,
                Err(e) => {
                    warn!("wl_shm create_pool mmap failed: {e}");
                    return;
                }
            };
            let _pool = data_init.init(id, ShmPoolData { memory });
        }
    }
}

impl wayland_server::Dispatch<WlShmPool, ShmPoolData> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &WlShmPool,
        request: wl_shm_pool::Request,
        pool: &ShmPoolData,
        _dhandle: &DisplayHandle,
        data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        match request {
            wl_shm_pool::Request::CreateBuffer {
                id,
                offset,
                width,
                height,
                stride,
                format: _,
            } => {
                let _buf = data_init.init(
                    id,
                    BufferData {
                        pool: pool.memory.clone(),
                        offset,
                        width,
                        height,
                        stride,
                    },
                );
            }
            wl_shm_pool::Request::Destroy => {}
            wl_shm_pool::Request::Resize { size: _ } => {}
            _ => {}
        }
    }
}

impl wayland_server::Dispatch<WlBuffer, BufferData> for CompositorState {
    fn request(
        _state: &mut Self,
        _client: &wayland_server::Client,
        _resource: &WlBuffer,
        request: wl_buffer::Request,
        _data: &BufferData,
        _dhandle: &DisplayHandle,
        _data_init: &mut wayland_server::DataInit<'_, Self>,
    ) {
        if let wl_buffer::Request::Destroy = request {
            // buffer dropped
        }
    }
}

fn map_shm_fd(fd: i32, size: usize) -> Result<Arc<[u8]>, String> {
    if size == 0 {
        return Err("zero-size shm pool".into());
    }
    let ptr = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, size) };
    let vec = slice.to_vec();
    unsafe {
        libc::munmap(ptr, size);
    }
    Ok(Arc::from(vec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_shm_rejects_zero_size() {
        assert!(map_shm_fd(-1, 0).is_err());
    }
}
