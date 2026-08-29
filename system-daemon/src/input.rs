//! Real evdev input forwarding with provenance markers (raw linux/input.h).

use common::ProvenanceMarker;
use serde_json::json;
use std::fs::{File, OpenOptions};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use uuid::Uuid;

const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const EV_ABS: u16 = 0x03;
const BTN_LEFT: u16 = 0x110;
const REL_X: u16 = 0x00;
const REL_Y: u16 = 0x01;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;

#[repr(C)]
struct InputEvent {
    time: libc::timeval,
    type_: u16,
    code: u16,
    value: i32,
}

pub struct InputForwarder {
    secret: [u8; 32],
}

impl InputForwarder {
    pub fn new() -> Self {
        let secret = std::env::var("THE_MACHINE_PROVENANCE_SECRET")
            .unwrap_or_else(|_| "the-machine-provenance-v1".into());
        let mut s = [0u8; 32];
        for (i, b) in secret.as_bytes().iter().take(32).enumerate() {
            s[i] = *b;
        }
        InputForwarder { secret: s }
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        if std::env::var("THE_MACHINE_INPUT_SIMULATE").ok().as_deref() == Some("1") {
            warn!("input simulation enabled");
            return self.simulated_loop().await;
        }
        let devices = discover_input_devices();
        if devices.is_empty() {
            warn!("no evdev devices found; enabling simulation");
            return self.simulated_loop().await;
        }
        info!("evdev: monitoring {} device(s)", devices.len());
        let secret = self.secret;
        tokio::task::spawn_blocking(move || run_evdev_loop(devices, secret));
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
        }
    }

    async fn simulated_loop(&self) -> anyhow::Result<()> {
        loop {
            let _ = forward_pointer(640, 360, "move", None).await;
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    }
}

fn discover_input_devices() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir("/dev/input") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |n| n.starts_with("event"))
            {
                out.push(path);
            }
        }
    }
    out
}

fn run_evdev_loop(devices: Vec<PathBuf>, secret: [u8; 32]) {
    use common::ProvenanceVerifier;
    let verifier = ProvenanceVerifier::new(secret);
    let mut x: i32 = 640;
    let mut y: i32 = 360;
    let mut seq: u64 = 0;
    let mut files: Vec<File> = devices
        .iter()
        .filter_map(|p| {
            OpenOptions::new()
                .read(true)
                .open(p)
                .ok()
        })
        .collect();
    if files.is_empty() {
        return;
    }
    loop {
        for file in &mut files {
            let mut ev = InputEvent {
                time: unsafe { std::mem::zeroed() },
                type_: 0,
                code: 0,
                value: 0,
            };
            let size = std::mem::size_of::<InputEvent>();
            let mut buf = unsafe {
                std::slice::from_raw_parts_mut(
                    &mut ev as *mut InputEvent as *mut u8,
                    size,
                )
            };
            match file.read_exact(&mut buf) {
                Ok(()) => dispatch_event(&ev, &verifier, &mut x, &mut y, &mut seq),
                Err(_) => continue,
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn dispatch_event(
    ev: &InputEvent,
    verifier: &common::ProvenanceVerifier,
    x: &mut i32,
    y: &mut i32,
    seq: &mut u64,
) {
    match ev.type_ {
        EV_REL => match ev.code {
            REL_X => *x += ev.value,
            REL_Y => *y += ev.value,
            _ => {}
        },
        EV_ABS => match ev.code {
            ABS_X => *x = ev.value,
            ABS_Y => *y = ev.value,
            _ => {}
        },
        EV_KEY if ev.code == BTN_LEFT && ev.value == 1 => {
            *seq += 1;
            let marker = verifier.generate_marker(
                ev.time.tv_sec as u64,
                13,
                0,
                *seq,
            );
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            if let Ok(rt) = rt {
                rt.block_on(forward_pointer(*x, *y, "click", Some(marker)));
            }
        }
        _ => {}
    }
}

async fn forward_pointer(
    x: i32,
    y: i32,
    event: &str,
    provenance: Option<ProvenanceMarker>,
) -> Option<()> {
    let mut params = json!({ "x": x, "y": y, "event": event });
    if let Some(p) = provenance {
        params["provenance"] = serde_json::to_value(p).ok()?;
        params["require_provenance"] = json!(true);
    }
    let hit = bus_call("compositor.input", params.clone()).await?;
    if let Some(widget) = hit.get("widget_id").and_then(|v| v.as_str()) {
        let _ = bus_call(
            "ui.event",
            json!({
                "id": widget,
                "event": event,
                "payload": { "x": x, "y": y, "provenance": params.get("provenance") },
            }),
        )
        .await;
    }
    Some(())
}

async fn bus_call(method: &str, params: serde_json::Value) -> Option<serde_json::Value> {
    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let path = format!("{}/mcp-bus.sock", socket_dir);
    let mut stream = tokio::net::UnixStream::connect(&path).await.ok()?;
    let req = json!({
        "id": Uuid::new_v4(),
        "kind": "Request",
        "method": method,
        "params": params,
    });
    let mut bytes = serde_json::to_vec(&req).ok()?;
    bytes.push(b'\n');
    stream.write_all(&bytes).await.ok()?;
    let mut buf = vec![0u8; 65536];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf).await.ok()?;
    let resp: serde_json::Value = serde_json::from_slice(&buf[..n]).ok()?;
    resp.get("result").cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_event_size_matches_linux() {
        // linux/input_event is 24 bytes on 64-bit.
        assert_eq!(std::mem::size_of::<InputEvent>(), 24);
    }
}
