//! In-memory clipboard for boot path (Ctrl/Cmd-C/V on fields).

use std::sync::Mutex;

static CLIPBOARD: Mutex<String> = Mutex::new(String::new());

pub fn get() -> String {
    CLIPBOARD.lock().map(|g| g.clone()).unwrap_or_default()
}

pub fn set(text: impl Into<String>) {
    if let Ok(mut g) = CLIPBOARD.lock() {
        *g = text.into();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        set("hello");
        assert_eq!(get(), "hello");
    }
}
