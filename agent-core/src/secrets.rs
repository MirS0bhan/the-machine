//! Production cloud API key loading — env vars and permission-checked secret files.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CloudSecret {
    pub api_key: String,
    pub source: String,
}

/// Load cloud API key from env or secret files (owner-only permissions).
pub fn load_cloud_api_key() -> Option<CloudSecret> {
    for (name, value) in [
        (
            "THE_MACHINE_CLOUD_API_KEY",
            std::env::var("THE_MACHINE_CLOUD_API_KEY").ok(),
        ),
        ("OPENAI_API_KEY", std::env::var("OPENAI_API_KEY").ok()),
    ] {
        if let Some(key) = value {
            let key = key.trim().to_string();
            if !key.is_empty() {
                return Some(CloudSecret {
                    api_key: key,
                    source: format!("env:{name}"),
                });
            }
        }
    }

    for path in secret_file_candidates() {
        if let Some(key) = read_secret_file(&path) {
            return Some(CloudSecret {
                api_key: key,
                source: format!("file:{}", path.display()),
            });
        }
    }
    None
}

fn secret_file_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Ok(p) = std::env::var("THE_MACHINE_CLOUD_API_KEY_FILE") {
        paths.push(PathBuf::from(p));
    }
    paths.push(PathBuf::from("/run/the-machine/secrets/cloud-api-key"));
    paths.push(PathBuf::from("/etc/the-machine/secrets/cloud-api-key"));
    paths
}

fn read_secret_file(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path).ok()?.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            tracing::warn!(
                "rejecting cloud API key file {:?}: permissions {:o} (must be owner-only)",
                path,
                mode
            );
            return None;
        }
    }
    let key = fs::read_to_string(path).ok()?.trim().to_string();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

pub fn cloud_key_status() -> serde_json::Value {
    match load_cloud_api_key() {
        Some(s) => {
            let prefix: String = s.api_key.chars().take(8).collect();
            serde_json::json!({
                "configured": true,
                "source": s.source,
                "key_prefix": prefix,
            })
        }
        None => serde_json::json!({
            "configured": false,
            "hint": "Set OPENAI_API_KEY, THE_MACHINE_CLOUD_API_KEY, or write key to /run/the-machine/secrets/cloud-api-key (mode 0600)",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn rejects_world_readable_secret_file() {
        let path = format!("/tmp/tm-secret-test-{}-bad", std::process::id());
        let mut f = fs::File::create(&path).unwrap();
        write!(f, "sk-test-key").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&path, perms).unwrap();
        assert!(read_secret_file(std::path::Path::new(&path)).is_none());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn accepts_owner_only_secret_file() {
        let path = format!("/tmp/tm-secret-test-{}-good", std::process::id());
        let mut f = fs::File::create(&path).unwrap();
        write!(f, "sk-test-key").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms).unwrap();
        assert_eq!(
            read_secret_file(std::path::Path::new(&path)).as_deref(),
            Some("sk-test-key")
        );
        let _ = fs::remove_file(&path);
    }
}
