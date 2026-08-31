//! Locale catalogs + string lookup.
//!
//! Resolution is a three-layer merge so a partial catalog never leaves a raw
//! `i18n:` id on screen: English builtin, then the language catalog
//! (`pt.json`), then the region catalog (`pt-BR.json`). Catalogs live in
//! `assets/locales/` and are installed to `/etc/the-machine/locales`.

use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

static CATALOGS: RwLock<Option<CatalogStore>> = RwLock::new(None);

#[derive(Clone, Debug, Default)]
struct CatalogStore {
    locale: String,
    strings: HashMap<String, String>,
    /// Which of the requested layers actually had a catalog on disk.
    sources: Vec<String>,
}

/// Locales whose layout mirrors and whose text runs right-to-left.
pub const RTL_LANGUAGES: [&str; 6] = ["ar", "fa", "he", "ur", "ps", "yi"];

/// Chrome keys every catalog is expected to carry.
pub const CHROME_KEYS: [&str; 15] = [
    "app.name",
    "app.welcome",
    "status.ready",
    "chat.placeholder",
    "chat.send",
    "chat.mic",
    "chat.suggestions",
    "chat.log",
    "workspace.hint",
    "dialog.approve",
    "dialog.deny",
    "dialog.dismiss",
    "activity.thinking",
    "activity.idle",
    "error.unavailable",
];

/// Normalize a POSIX / BCP-47 locale spelling to `lang` or `lang-REGION`.
///
/// `pt_BR.UTF-8@euro` → `pt-BR`, `EN` → `en`, `ca_ES@valencia` → `ca-valencia`.
pub fn normalize_locale(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return "en".into();
    }
    let (base, modifier) = match raw.split_once('@') {
        Some((b, m)) => (b, Some(m)),
        None => (raw, None),
    };
    let base = base.split('.').next().unwrap_or(base);
    let mut parts = base.split(['_', '-']);
    let lang = parts.next().unwrap_or("en").to_lowercase();
    // A modifier like @valencia names a variant, which wins over the region.
    if let Some(m) = modifier.filter(|m| !m.is_empty()) {
        return format!("{lang}-{}", m.to_lowercase());
    }
    match parts.next().filter(|r| !r.is_empty()) {
        // Two-letter / three-digit subtags are regions and are upper-cased;
        // anything longer is a variant (`qps-ploc`) and stays lower-case.
        Some(sub)
            if sub.len() == 2 || (sub.len() == 3 && sub.chars().all(|c| c.is_ascii_digit())) =>
        {
            format!("{lang}-{}", sub.to_uppercase())
        }
        Some(sub) => format!("{lang}-{}", sub.to_lowercase()),
        None => lang,
    }
}

/// Language subtag of a locale (`pt-BR` → `pt`).
pub fn language_of(locale: &str) -> String {
    locale.split('-').next().unwrap_or(locale).to_lowercase()
}

pub fn default_locale() -> String {
    let raw = std::env::var("THE_MACHINE_LOCALE")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "en".into());
    normalize_locale(&raw)
}

pub fn ensure_loaded() {
    if CATALOGS.read().ok().and_then(|g| g.clone()).is_some() {
        return;
    }
    let locale = default_locale();
    let _ = load_locale(&locale);
}

/// Directories searched for `<locale>.json`, most specific last.
fn catalog_dirs() -> Vec<String> {
    let manifest_locales = format!("{}/../assets/locales", env!("CARGO_MANIFEST_DIR"));
    vec![
        "/etc/the-machine/locales".into(),
        manifest_locales,
        std::env::var("THE_MACHINE_LOCALE_DIR").unwrap_or_default(),
    ]
}

fn read_catalog(locale: &str) -> Option<HashMap<String, String>> {
    let mut found: Option<HashMap<String, String>> = None;
    for dir in catalog_dirs() {
        if dir.is_empty() {
            continue;
        }
        let path = std::path::Path::new(&dir).join(format!("{locale}.json"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&text) {
            let entry = found.get_or_insert_with(HashMap::new);
            for (k, v) in map {
                if let Some(s) = v.as_str() {
                    entry.insert(k, s.to_string());
                }
            }
        }
    }
    found
}

pub fn load_locale(locale: &str) -> Result<(), String> {
    let locale = normalize_locale(locale);
    let language = language_of(&locale);
    let mut strings = builtin_catalog();
    let mut sources = vec!["builtin:en".to_string()];
    // Language layer, then region layer (`pt` under `pt-BR`).
    for layer in [language.clone(), locale.clone()] {
        if layer == "en" && sources.len() == 1 {
            continue;
        }
        if let Some(map) = read_catalog(&layer) {
            for (k, v) in map {
                strings.insert(k, v);
            }
            sources.push(layer);
        }
    }
    let mut g = CATALOGS.write().map_err(|e| e.to_string())?;
    *g = Some(CatalogStore {
        locale,
        strings,
        sources,
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

/// Catalog layers that were actually found, for honest status reporting.
pub fn loaded_sources() -> Vec<String> {
    ensure_loaded();
    CATALOGS
        .read()
        .ok()
        .and_then(|g| g.as_ref().map(|c| c.sources.clone()))
        .unwrap_or_default()
}

/// Locales with a catalog on disk.
pub fn available_locales() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for dir in catalog_dirs() {
        if dir.is_empty() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(".json") {
                if !out.iter().any(|l| l == stem) {
                    out.push(stem.to_string());
                }
            }
        }
    }
    out.sort();
    out
}

/// Keys the active catalog is missing relative to the chrome contract.
pub fn missing_chrome_keys() -> Vec<&'static str> {
    ensure_loaded();
    let guard = CATALOGS.read().ok();
    let strings = guard.as_ref().and_then(|g| g.as_ref().map(|c| &c.strings));
    CHROME_KEYS
        .iter()
        .filter(|k| strings.map(|s| !s.contains_key(**k)).unwrap_or(true))
        .copied()
        .collect()
}

pub fn status() -> Value {
    ensure_loaded();
    let locale = current_locale();
    serde_json::json!({
        "locale": locale,
        "language": language_of(&locale),
        "keys": CATALOGS.read().ok().and_then(|g| g.as_ref().map(|c| c.strings.len())).unwrap_or(0),
        "sources": loaded_sources(),
        "rtl": is_rtl_locale(&locale),
        "rtl_locales": RTL_LANGUAGES,
        "available": available_locales(),
        "missing_chrome_keys": missing_chrome_keys(),
    })
}

pub fn is_rtl_locale(locale: &str) -> bool {
    RTL_LANGUAGES.contains(&language_of(locale).as_str())
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

/// English strings compiled in, so the shell still reads correctly when no
/// catalog files are installed at all.
fn builtin_catalog() -> HashMap<String, String> {
    [
        ("app.name", "The Machine"),
        ("app.welcome", "Welcome back"),
        ("status.ready", "The Machine · session ready"),
        ("chat.placeholder", "Ask or say what you need"),
        ("chat.send", "Send"),
        ("chat.mic", "Dictate"),
        ("chat.suggestions", "Suggestions"),
        ("chat.log", "Conversation"),
        ("workspace.hint", "Ask me to build something here"),
        ("dialog.approve", "Approve"),
        ("dialog.deny", "Deny"),
        ("dialog.dismiss", "Dismiss"),
        ("activity.thinking", "Working…"),
        ("activity.idle", "Ready"),
        ("error.unavailable", "Not available on this machine"),
    ]
    .iter()
    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The catalog store is process-global; serialise tests that swap locale.
    fn guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        match LOCK.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        }
    }

    #[test]
    fn english_welcome() {
        let _g = guard();
        load_locale("en").unwrap();
        assert_eq!(t("app.welcome"), "Welcome back");
    }

    #[test]
    fn rtl_languages_detected_with_and_without_region() {
        assert!(is_rtl_locale("fa"));
        assert!(is_rtl_locale("ar-EG"));
        assert!(is_rtl_locale("he"));
        assert!(!is_rtl_locale("en"));
        assert!(!is_rtl_locale("pt-BR"));
    }

    #[test]
    fn posix_locale_spellings_normalize() {
        assert_eq!(normalize_locale("pt_BR.UTF-8"), "pt-BR");
        assert_eq!(normalize_locale("EN"), "en");
        assert_eq!(normalize_locale("zh_TW"), "zh-TW");
        assert_eq!(normalize_locale("ca_ES@valencia"), "ca-valencia");
        assert_eq!(normalize_locale(""), "en");
        assert_eq!(language_of("pt-BR"), "pt");
    }

    #[test]
    fn every_shipped_catalog_resolves_chrome_keys() {
        let _g = guard();
        let locales = available_locales();
        assert!(
            locales.len() >= 40,
            "expected many catalogs, got {locales:?}"
        );
        for locale in locales {
            load_locale(&locale).unwrap();
            assert!(
                missing_chrome_keys().is_empty(),
                "{locale} is missing {:?}",
                missing_chrome_keys()
            );
            for key in CHROME_KEYS {
                assert_ne!(t(key), key, "{locale} leaked raw key {key}");
            }
        }
        load_locale("en").unwrap();
    }

    #[test]
    fn region_catalog_layers_over_language() {
        let _g = guard();
        load_locale("pt-BR").unwrap();
        // pt-BR overrides `activity.thinking`; the rest comes from pt.
        assert_eq!(t("activity.thinking"), "Trabalhando…");
        assert_eq!(t("chat.send"), "Enviar");
        assert!(loaded_sources().iter().any(|s| s == "pt"));
        assert!(loaded_sources().iter().any(|s| s == "pt-BR"));
        load_locale("en").unwrap();
    }

    #[test]
    fn unknown_locale_falls_back_to_english() {
        let _g = guard();
        load_locale("xx-YY").unwrap();
        assert_eq!(t("chat.send"), "Send");
        assert_eq!(current_locale(), "xx-YY");
        assert_eq!(loaded_sources(), vec!["builtin:en"]);
        load_locale("en").unwrap();
    }

    #[test]
    fn pseudo_locale_expands_strings_for_overflow_testing() {
        let _g = guard();
        load_locale("qps-ploc").unwrap();
        let send = t("chat.send");
        assert!(
            send.len() > "Send".len(),
            "pseudo-loc must be longer: {send}"
        );
        load_locale("en").unwrap();
    }

    #[test]
    fn labels_resolve_through_prefixes() {
        let _g = guard();
        load_locale("en").unwrap();
        assert_eq!(resolve_label("i18n:chat.send"), "Send");
        assert_eq!(resolve_label("@chat.send"), "Send");
        assert_eq!(resolve_label("Plain"), "Plain");
    }
}
