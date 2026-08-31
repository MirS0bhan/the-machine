//! Locale catalogs + string lookup (P2 i18n).

use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

static CATALOGS: RwLock<Option<CatalogStore>> = RwLock::new(None);

#[derive(Clone, Debug, Default)]
struct CatalogStore {
    locale: String,
    strings: HashMap<String, String>,
}

pub fn default_locale() -> String {
    std::env::var("THE_MACHINE_LOCALE")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en".into())
        .split('.')
        .next()
        .unwrap_or("en")
        .split('_')
        .next()
        .unwrap_or("en")
        .to_lowercase()
}

pub fn ensure_loaded() {
    if CATALOGS.read().ok().and_then(|g| g.clone()).is_some() {
        return;
    }
    let locale = default_locale();
    let _ = load_locale(&locale);
}

pub fn load_locale(locale: &str) -> Result<(), String> {
    let mut strings = builtin_catalog("en");
    if locale != "en" {
        for (k, v) in builtin_catalog(locale) {
            strings.insert(k, v);
        }
    }
    // Optional on-disk override: assets/locales/{locale}.json or /etc/the-machine/locales/
    let manifest_locales = format!("{}/../assets/locales", env!("CARGO_MANIFEST_DIR"));
    for dir in [
        std::env::var("THE_MACHINE_LOCALE_DIR").unwrap_or_default(),
        "/etc/the-machine/locales".into(),
        manifest_locales,
        "/workspace/assets/locales".into(),
    ] {
        if dir.is_empty() {
            continue;
        }
        let path = std::path::Path::new(&dir).join(format!("{locale}.json"));
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&text) {
                for (k, v) in map {
                    if let Some(s) = v.as_str() {
                        strings.insert(k, s.to_string());
                    }
                }
            }
        }
    }
    let mut g = CATALOGS.write().map_err(|e| e.to_string())?;
    *g = Some(CatalogStore {
        locale: locale.to_string(),
        strings,
    });
    Ok(())
}

pub fn t(key: &str) -> String {
    ensure_loaded();
    CATALOGS
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.strings.get(key).cloned()))
        .flatten()
        .unwrap_or_else(|| key.to_string())
}

pub fn current_locale() -> String {
    ensure_loaded();
    CATALOGS
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.locale.clone()))
        .unwrap_or_else(default_locale)
}

pub fn status() -> Value {
    ensure_loaded();
    serde_json::json!({
        "locale": current_locale(),
        "keys": CATALOGS.read().ok().and_then(|g| g.as_ref().map(|c| c.strings.len())).unwrap_or(0),
        "rtl_locales": ["ar", "fa", "he", "ur"],
    })
}

pub fn is_rtl_locale(locale: &str) -> bool {
    matches!(locale, "ar" | "fa" | "he" | "ur")
}

/// Resolve `i18n:key` / `@domain.key` labels; pass through plain strings.
pub fn resolve_label(s: &str) -> String {
    if let Some(key) = s.strip_prefix("i18n:") {
        return t(key);
    }
    if let Some(key) = s.strip_prefix('@') {
        if key.contains('.') {
            return t(key);
        }
    }
    s.to_string()
}

/// True when the active locale should mirror layout by default.
pub fn active_rtl() -> bool {
    is_rtl_locale(&current_locale())
}

fn builtin_catalog(locale: &str) -> HashMap<String, String> {
    let pairs: &[(&str, &str)] = match locale {
        "fa" => &[
            ("app.welcome", "خوش آمدید"),
            ("chat.placeholder", "بپرسید یا بگویید چه نیاز دارید"),
            ("chat.send", "ارسال"),
            ("dialog.approve", "تأیید"),
            ("dialog.deny", "رد"),
        ],
        "ar" => &[
            ("app.welcome", "مرحباً بعودتك"),
            ("chat.placeholder", "اسأل أو قل ما تحتاجه"),
            ("chat.send", "إرسال"),
            ("dialog.approve", "موافقة"),
            ("dialog.deny", "رفض"),
        ],
        _ => &[
            ("app.welcome", "Welcome back"),
            ("chat.placeholder", "Ask or say what you need"),
            ("chat.send", "Send"),
            ("dialog.approve", "Approve"),
            ("dialog.deny", "Deny"),
        ],
    };
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_welcome() {
        load_locale("en").unwrap();
        assert_eq!(t("app.welcome"), "Welcome back");
    }

    #[test]
    fn fa_is_rtl() {
        assert!(is_rtl_locale("fa"));
        assert!(!is_rtl_locale("en"));
    }
}
