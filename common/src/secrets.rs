//! Secret file locations and safe reads (Wi-Fi credentials, tokens, API keys).

use std::path::{Path, PathBuf};

/// Search order for secret files on boot and installed systems.
pub const SECRET_DIRS: &[&str] = &["/run/the-machine/secrets", "/etc/the-machine/secrets"];

/// Reject path traversal in credential references.
pub fn validate_secret_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("secret name is required".into());
    }
    if name.contains('/') || name.contains("..") {
        return Err("invalid secret name".into());
    }
    Ok(())
}

/// Resolve `{dir}/{name}` for each [`SECRET_DIRS`] entry.
pub fn secret_paths(name: &str) -> Result<Vec<PathBuf>, String> {
    validate_secret_name(name)?;
    Ok(SECRET_DIRS
        .iter()
        .map(|dir| Path::new(dir).join(name))
        .collect())
}

/// Read a secret by reference name from the first matching file in [`SECRET_DIRS`].
pub fn read_secret_by_name(name: &str, require_owner_only: bool) -> Result<String, String> {
    for path in secret_paths(name)? {
        if let Some(body) = read_secret_file(&path, require_owner_only) {
            return Ok(body);
        }
    }
    Err(format!("secret not found in {}", SECRET_DIRS.join(" or ")))
}

/// Read a secret file with optional owner-only (0600) permission enforcement.
pub fn read_secret_file(path: &Path, require_owner_only: bool) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    #[cfg(unix)]
    if require_owner_only {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(path).ok()?.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            return None;
        }
    }
    let body = std::fs::read_to_string(path).ok()?;
    let trimmed = body.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_traversal() {
        assert!(validate_secret_name("../etc/passwd").is_err());
    }
}
