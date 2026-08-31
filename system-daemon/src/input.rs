//! Real evdev input forwarding with provenance markers (raw linux/input.h).

use common::ProvenanceMarker;
use serde_json::json;
use std::fs::{File, OpenOptions};
use std::io::Read;
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
const REL_WHEEL: u16 = 0x08;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const KEY_LEFTSHIFT: u16 = 42;
const KEY_RIGHTSHIFT: u16 = 54;
const KEY_LEFTCTRL: u16 = 29;
const KEY_RIGHTCTRL: u16 = 97;
const KEY_LEFTALT: u16 = 56;
const KEY_RIGHTALT: u16 = 100;
const KEY_BACKSPACE: u16 = 14;
const KEY_TAB: u16 = 15;
const KEY_ENTER: u16 = 28;
const KEY_ESC: u16 = 1;
const KEY_DELETE: u16 = 111;
const KEY_LEFT: u16 = 105;
const KEY_RIGHT: u16 = 106;

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
                .is_some_and(|n| n.starts_with("event"))
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
    let mut shift = false;
    let mut ctrl = false;
    let mut alt = false;
    let mut files: Vec<File> = devices
        .iter()
        .filter_map(|p| OpenOptions::new().read(true).open(p).ok())
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
            let buf = unsafe {
                std::slice::from_raw_parts_mut(&mut ev as *mut InputEvent as *mut u8, size)
            };
            match file.read_exact(buf) {
                Ok(()) => dispatch_event(
                    &ev,
                    &verifier,
                    &mut x,
                    &mut y,
                    &mut seq,
                    &mut shift,
                    &mut ctrl,
                    &mut alt,
                ),
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
    shift: &mut bool,
    ctrl: &mut bool,
    alt: &mut bool,
) {
    match ev.type_ {
        EV_REL => match ev.code {
            REL_X => {
                *x += ev.value;
                forward_move_throttled(*x, *y);
            }
            REL_Y => {
                *y += ev.value;
                forward_move_throttled(*x, *y);
            }
            REL_WHEEL => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    // evdev wheel: positive = up; convert to pixel-ish delta.
                    let dy = -ev.value * 40;
                    rt.block_on(forward_wheel(*x, *y, dy));
                }
            }
            _ => {}
        },
        EV_ABS => match ev.code {
            ABS_X => *x = ev.value,
            ABS_Y => *y = ev.value,
            _ => {}
        },
        EV_KEY if ev.code == BTN_LEFT => {
            if ev.value == 1 {
                *seq += 1;
                let marker = verifier.generate_marker(ev.time.tv_sec as u64, 13, 0, *seq);
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    // Click carries provenance; ui-runtime applies local press chrome
                    // before bindings. Release clears pressed state.
                    rt.block_on(forward_pointer(*x, *y, "click", Some(marker)));
                }
            } else if ev.value == 0 {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                if let Ok(rt) = rt {
                    rt.block_on(forward_pointer(*x, *y, "release", None));
                }
            }
        }
        EV_KEY => {
            match ev.code {
                KEY_LEFTSHIFT | KEY_RIGHTSHIFT => *shift = ev.value != 0,
                KEY_LEFTCTRL | KEY_RIGHTCTRL => *ctrl = ev.value != 0,
                KEY_LEFTALT | KEY_RIGHTALT => *alt = ev.value != 0,
                _ if ev.value == 1 || ev.value == 2 => {
                    if let Some((key, text)) = map_keycode(ev.code, *shift) {
                        let rt = tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build();
                        if let Ok(rt) = rt {
                            rt.block_on(forward_key(key, text, *shift, *ctrl, *alt));
                        }
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
}

fn map_keycode(code: u16, shift: bool) -> Option<(String, Option<String>)> {
    match code {
        KEY_BACKSPACE => Some(("BackSpace".into(), None)),
        KEY_TAB => Some(("Tab".into(), None)),
        KEY_ENTER => Some(("Enter".into(), None)),
        KEY_ESC => Some(("Escape".into(), None)),
        KEY_DELETE => Some(("Delete".into(), None)),
        KEY_LEFT => Some(("ArrowLeft".into(), None)),
        KEY_RIGHT => Some(("ArrowRight".into(), None)),
        // Letter keys (US QWERTY scancodes 16–25, 30–38, 44–50).
        16..=25 | 30..=38 | 44..=50 => {
            const ROW1: &[u8] = b"qwertyuiop";
            const ROW2: &[u8] = b"asdfghjkl";
            const ROW3: &[u8] = b"zxcvbnm";
            let ch = if (16..=25).contains(&code) {
                ROW1[(code - 16) as usize] as char
            } else if (30..=38).contains(&code) {
                ROW2[(code - 30) as usize] as char
            } else {
                ROW3[(code - 44) as usize] as char
            };
            let ch = if shift {
                ch.to_ascii_uppercase()
            } else {
                ch
            };
            Some((ch.to_string(), Some(ch.to_string())))
        }
        57 => Some((" ".into(), Some(" ".into()))), // space
        2..=10 => {
            let digit = (b'0' + (code - 1) as u8) as char;
            let shifted = b")!@#$%^&*"[((code - 2) as usize).min(8)] as char;
            let ch = if shift { shifted } else { digit };
            Some((ch.to_string(), Some(ch.to_string())))
        }
        11 => {
            let ch = if shift { ')' } else { '0' };
            Some((ch.to_string(), Some(ch.to_string())))
        }
        _ => None,
    }
}

async fn forward_wheel(x: i32, y: i32, delta_y: i32) {
    let params = json!({ "x": x, "y": y, "event": "wheel", "delta_y": delta_y });
    let hit = bus_call("compositor.input", params.clone()).await;
    let widget = hit
        .as_ref()
        .and_then(|h| h.get("widget_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if !widget.is_empty() {
        let _ = bus_call(
            "ui.event",
            json!({
                "id": widget,
                "event": "wheel",
                "payload": { "x": x, "y": y, "delta_y": delta_y },
            }),
        )
        .await;
    }
}

fn forward_move_throttled(x: i32, y: i32) {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static LAST_MS: AtomicU64 = AtomicU64::new(0);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let prev = LAST_MS.load(Ordering::Relaxed);
    if now.saturating_sub(prev) < 32 {
        return;
    }
    LAST_MS.store(now, Ordering::Relaxed);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build();
    if let Ok(rt) = rt {
        rt.block_on(forward_pointer(x, y, "move", None));
    }
}

async fn forward_key(key: String, text: Option<String>, shift: bool, ctrl: bool, alt: bool) {
    let params = json!({
        "event": "key",
        "key": key,
        "text": text,
        "shift": shift,
        "ctrl": ctrl,
        "alt": alt,
    });
    let hit = bus_call("compositor.input", params.clone()).await;
    let widget = hit
        .as_ref()
        .and_then(|h| h.get("widget_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("ui.chat_input");
    let _ = bus_call(
        "ui.event",
        json!({
            "id": widget,
            "event": "key",
            "payload": {
                "key": key,
                "text": text,
                "shift": shift,
                "ctrl": ctrl,
                "alt": alt,
            },
        }),
    )
    .await;
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
        let mut payload = json!({ "x": x, "y": y, "provenance": params.get("provenance") });
        if let Some(geo) = hit.get("geometry") {
            payload["geometry"] = geo.clone();
        }
        if let Some(kind) = hit.get("kind") {
            payload["kind"] = kind.clone();
        }
        let _ = bus_call(
            "ui.event",
            json!({
                "id": widget,
                "event": event,
                "payload": payload,
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
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
        .await
        .ok()?;
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
