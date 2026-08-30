//! udev hotplug monitoring via kernel uevent netlink → `event.publish` on the Event Bus.

use serde_json::json;
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

const NETLINK_KOBJECT_UEVENT: i32 = 15;
const UEVENT_BUF_SIZE: usize = 32 * 1024;
const WATCHED_SUBSYSTEMS: &[&str] = &["input", "drm", "sound", "net"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotplugEvent {
    pub action: String,
    pub subsystem: String,
    pub device_path: String,
    pub device_node: Option<String>,
    pub properties: HashMap<String, String>,
}

impl HotplugEvent {
    pub fn event_pattern(&self) -> String {
        format!("hardware.hotplug.{}.{}", self.subsystem, self.action)
    }

    pub fn to_payload(&self) -> serde_json::Value {
        json!({
            "action": self.action,
            "subsystem": self.subsystem,
            "device_path": self.device_path,
            "device_node": self.device_node,
            "properties": self.properties,
        })
    }
}

#[allow(dead_code)]
pub fn hotplug_event_from_parts(
    action: &str,
    subsystem: &str,
    device_path: &str,
    device_node: Option<&str>,
    properties: HashMap<String, String>,
) -> Option<HotplugEvent> {
    if !WATCHED_SUBSYSTEMS.contains(&subsystem) {
        return None;
    }
    Some(HotplugEvent {
        action: action.to_string(),
        subsystem: subsystem.to_string(),
        device_path: device_path.to_string(),
        device_node: device_node.map(str::to_string),
        properties,
    })
}

/// Parse a kernel uevent buffer (`action@/devices/...\\0KEY=val\\0...`).
pub fn parse_uevent_buffer(buf: &[u8]) -> Option<HotplugEvent> {
    let text = std::str::from_utf8(buf).ok()?;
    let header = text.split('\0').next()?;
    let (action, device_path) = header.split_once('@')?;
    if action.is_empty() || device_path.is_empty() {
        return None;
    }

    let mut fields: HashMap<String, String> = HashMap::new();
    for part in text.split('\0').skip(1) {
        if part.is_empty() {
            continue;
        }
        if let Some((key, value)) = part.split_once('=') {
            fields.insert(key.to_string(), value.to_string());
        }
    }

    let subsystem = fields.get("SUBSYSTEM")?.clone();
    if !WATCHED_SUBSYSTEMS.contains(&subsystem.as_str()) {
        return None;
    }

    let device_node = fields.get("DEVNAME").map(|dev| {
        if dev.starts_with('/') {
            dev.clone()
        } else {
            format!("/dev/{dev}")
        }
    });
    let mut properties = HashMap::new();
    for key in ["ID_MODEL", "ID_VENDOR", "INTERFACE"] {
        if let Some(v) = fields.get(key) {
            properties.insert(key.to_string(), v.clone());
        }
    }

    Some(HotplugEvent {
        action: action.to_string(),
        subsystem,
        device_path: device_path.to_string(),
        device_node,
        properties,
    })
}

pub async fn publish_hotplug_event(event: &HotplugEvent) {
    let socket_path = common::component_socket("event-bus");
    let Ok(mut stream) = tokio::net::UnixStream::connect(&socket_path).await else {
        return;
    };
    let req = json!({
        "id": uuid::Uuid::new_v4(),
        "kind": "Request",
        "method": "event.publish",
        "params": {
            "category": "external",
            "pattern": event.event_pattern(),
            "source": "system-daemon",
            "payload": event.to_payload(),
        }
    });
    let mut bytes = serde_json::to_vec(&req).unwrap_or_default();
    bytes.push(b'\n');
    let _ = stream.write_all(&bytes).await;
}

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_HOTPLUG").is_ok() {
        info!("hotplug monitor disabled");
        return;
    }
    tokio::task::spawn_blocking(run_monitor);
}

fn open_uevent_socket() -> Result<std::os::fd::RawFd, String> {
    let fd = unsafe {
        libc::socket(
            libc::AF_NETLINK,
            libc::SOCK_DGRAM | libc::SOCK_CLOEXEC,
            NETLINK_KOBJECT_UEVENT,
        )
    };
    if fd < 0 {
        return Err(format!(
            "uevent socket: {}",
            std::io::Error::last_os_error()
        ));
    }

    let mut addr: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
    addr.nl_family = libc::AF_NETLINK as u16;
    addr.nl_pid = std::process::id() as u32;
    addr.nl_groups = 1;
    let ret = unsafe {
        libc::bind(
            fd,
            &addr as *const _ as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_nl>() as u32,
        )
    };
    if ret < 0 {
        unsafe {
            libc::close(fd);
        }
        return Err(format!("uevent bind: {}", std::io::Error::last_os_error()));
    }
    Ok(fd)
}

fn run_monitor() {
    let fd = match open_uevent_socket() {
        Ok(fd) => fd,
        Err(e) => {
            warn!("uevent monitor unavailable: {e}");
            return;
        }
    };
    info!(
        "kernel uevent hotplug monitor active for {:?}",
        WATCHED_SUBSYSTEMS
    );

    let mut buf = vec![0u8; UEVENT_BUF_SIZE];
    loop {
        let n = unsafe { libc::recv(fd, buf.as_mut_ptr() as *mut _, buf.len(), 0) };
        if n <= 0 {
            warn!("uevent recv ended: {}", std::io::Error::last_os_error());
            break;
        }
        let Some(hotplug) = parse_uevent_buffer(&buf[..n as usize]) else {
            continue;
        };
        info!(
            "hotplug {} {} {:?}",
            hotplug.subsystem, hotplug.action, hotplug.device_node
        );
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        if let Ok(rt) = rt {
            rt.block_on(publish_hotplug_event(&hotplug));
        }
    }
    unsafe {
        libc::close(fd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_input_add_uevent() {
        let raw = b"add@/devices/pci0000:00/input/input5\0ACTION=add\0DEVPATH=/devices/pci0000:00/input/input5\0SUBSYSTEM=input\0DEVNAME=input/event12\0";
        let event = parse_uevent_buffer(raw).expect("parsed");
        assert_eq!(event.action, "add");
        assert_eq!(event.subsystem, "input");
        assert_eq!(event.event_pattern(), "hardware.hotplug.input.add");
        assert_eq!(event.device_node.as_deref(), Some("/dev/input/event12"));
    }

    #[test]
    fn builds_pattern_for_input_add() {
        let event = hotplug_event_from_parts(
            "add",
            "input",
            "/devices/pci0000:00/input/input5",
            Some("/dev/input/event12"),
            HashMap::new(),
        )
        .expect("input is watched");
        assert_eq!(event.event_pattern(), "hardware.hotplug.input.add");
        let payload = event.to_payload();
        assert_eq!(payload["action"], "add");
        assert_eq!(payload["device_node"], "/dev/input/event12");
    }

    #[test]
    fn ignores_unwatched_subsystems() {
        assert!(hotplug_event_from_parts(
            "add",
            "usb",
            "/devices/pci0000:00/usb1",
            None,
            HashMap::new(),
        )
        .is_none());
        let raw = b"add@/devices/usb1\0SUBSYSTEM=usb\0";
        assert!(parse_uevent_buffer(raw).is_none());
    }

    #[test]
    fn drm_remove_event_shape() {
        let event = hotplug_event_from_parts(
            "remove",
            "drm",
            "/devices/pci0000:00/drm/card0",
            Some("/dev/dri/card0"),
            HashMap::from([("ID_MODEL".into(), "GPU".into())]),
        )
        .expect("drm is watched");
        assert_eq!(event.event_pattern(), "hardware.hotplug.drm.remove");
        assert_eq!(event.to_payload()["properties"]["ID_MODEL"], "GPU");
    }
}
