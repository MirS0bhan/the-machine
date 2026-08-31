//! Clipboard: in-memory store with best-effort OS bridge (wl-copy / xclip / xsel).

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Mutex;

static CLIPBOARD: Mutex<String> = Mutex::new(String::new());

pub fn get() -> String {
    if let Some(os) = os_get() {
        if let Ok(mut g) = CLIPBOARD.lock() {
            *g = os.clone();
        }
        return os;
    }
    CLIPBOARD.lock().map(|g| g.clone()).unwrap_or_default()
}

pub fn set(text: impl Into<String>) {
    let text = text.into();
    if let Ok(mut g) = CLIPBOARD.lock() {
        *g = text.clone();
    }
    let _ = os_set(&text);
}

fn os_get() -> Option<String> {
    for (bin, args) in [
        ("wl-paste", vec!["--no-newline"]),
        ("xclip", vec!["-selection", "clipboard", "-o"]),
        ("xsel", vec!["--clipboard", "--output"]),
    ] {
        if let Ok(out) = Command::new(bin).args(&args).output() {
            if out.status.success() {
                return Some(String::from_utf8_lossy(&out.stdout).to_string());
            }
        }
    }
    None
}

fn os_set(text: &str) -> bool {
    for (bin, args) in [
        ("wl-copy", vec![] as Vec<&str>),
        ("xclip", vec!["-selection", "clipboard"]),
        ("xsel", vec!["--clipboard", "--input"]),
    ] {
        if let Ok(mut child) = Command::new(bin)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().map(|s| s.success()).unwrap_or(false) {
                return true;
            }
        }
    }
    false
}

pub fn backend_name() -> &'static str {
    if which("wl-copy") || which("wl-paste") {
        "wayland+memory"
    } else if which("xclip") || which("xsel") {
        "x11+memory"
    } else {
        "memory"
    }
}

fn which(bin: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {bin} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_memory() {
        set("hello-p2");
        // Memory store always updated; OS tools may or may not be present.
        assert_eq!(
            CLIPBOARD.lock().map(|g| g.clone()).unwrap_or_default(),
            "hello-p2"
        );
    }
}
