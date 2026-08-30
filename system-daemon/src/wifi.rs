//! Wi-Fi connect via `wpa_cli` + owner-only credential files (G14).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::net;

const WIFI_SECRETS_RUN: &str = "/run/the-machine/secrets/wifi";
const WIFI_SECRETS_ETC: &str = "/etc/the-machine/secrets/wifi";

/// Resolve a PSK from `credential_ref` (basename only) under the wifi secrets dir.
pub fn resolve_credential(credential_ref: &str) -> Result<String, String> {
    let ref_name = credential_ref.trim();
    if ref_name.is_empty() {
        return Err("missing credential_ref (wifi PSK secret id)".into());
    }
    if ref_name.contains('/') || ref_name.contains("..") {
        return Err("invalid credential_ref (must be a basename)".into());
    }
    for base in [WIFI_SECRETS_RUN, WIFI_SECRETS_ETC] {
        let path = PathBuf::from(base).join(ref_name);
        if let Some(psk) = read_owner_only_secret(&path) {
            return Ok(psk);
        }
    }
    Err(format!(
        "wifi credential not found for ref '{ref_name}' (expected {WIFI_SECRETS_RUN}/<ref> mode 0600)"
    ))
}

fn read_owner_only_secret(path: &Path) -> Option<String> {
    if !path.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(path).ok()?.permissions().mode() & 0o777;
        if mode & 0o077 != 0 {
            tracing::warn!(
                "rejecting wifi secret {:?}: permissions {:o} (must be owner-only)",
                path,
                mode
            );
            return None;
        }
    }
    let psk = fs::read_to_string(path).ok()?.trim().to_string();
    if psk.is_empty() {
        None
    } else {
        Some(psk)
    }
}

fn wpa_cli_path() -> PathBuf {
    std::env::var("THE_MACHINE_WPA_CLI")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("wpa_cli"))
}

fn pick_wifi_interface() -> Option<String> {
    net::list_interfaces()
        .into_iter()
        .find(|i| i.r#type == "wifi")
        .map(|i| i.name)
}

/// Connect to `ssid` using `credential_ref` → owner-only secret file + `wpa_cli`.
pub async fn connect_wifi(ssid: &str, credential_ref: &str) -> Result<String, String> {
    let ssid = ssid.trim();
    if ssid.is_empty() {
        return Err("missing ssid".into());
    }
    let psk = resolve_credential(credential_ref)?;

    let iface = pick_wifi_interface().ok_or_else(|| {
        "no wireless interface found (wpa_supplicant adapter requires wifi netdev)".to_string()
    })?;

    let wpa_cli = wpa_cli_path();
    if !command_exists(&wpa_cli) {
        return Err(format!(
            "wpa_cli not available at {} (install wpa_supplicant or set THE_MACHINE_WPA_CLI)",
            wpa_cli.display()
        ));
    }

    let ssid_owned = ssid.to_string();
    tokio::task::spawn_blocking(move || wpa_cli_connect(&wpa_cli, &iface, &ssid_owned, &psk))
        .await
        .map_err(|e| format!("wifi connect task failed: {e}"))?
}

fn command_exists(path: &Path) -> bool {
    if path.is_absolute() || path.components().count() > 1 {
        return path.is_file();
    }
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {}", path.to_string_lossy()))
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn wpa_cli_connect(wpa_cli: &Path, iface: &str, ssid: &str, psk: &str) -> Result<String, String> {
    let net_id = run_wpa(wpa_cli, iface, &["add_network"])?;
    let net_id = net_id.trim();
    if net_id.is_empty() || net_id.contains("FAIL") {
        return Err(format!("wpa_cli add_network failed: {net_id}"));
    }

    run_wpa(wpa_cli, iface, &["set_network", net_id, "ssid", ssid])?;
    run_wpa(wpa_cli, iface, &["set_network", net_id, "psk", psk])?;
    run_wpa(wpa_cli, iface, &["enable_network", net_id])?;
    run_wpa(wpa_cli, iface, &["select_network", net_id])?;
    run_wpa(wpa_cli, iface, &["reconnect"])?;

    Ok(format!("connecting:{ssid}"))
}

fn run_wpa(wpa_cli: &Path, iface: &str, args: &[&str]) -> Result<String, String> {
    let mut cmd = Command::new(wpa_cli);
    cmd.arg("-i").arg(iface);
    for a in args {
        cmd.arg(a);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("wpa_cli {} failed: {e}", args.join(" ")))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !output.status.success() || stdout.contains("FAIL") {
        let detail = if stderr.is_empty() {
            stdout.clone()
        } else {
            format!("{stdout} {stderr}")
        };
        return Err(format!("wpa_cli {} on {iface}: {detail}", args.join(" ")));
    }
    Ok(stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn rejects_empty_credential_ref() {
        assert!(resolve_credential("")
            .unwrap_err()
            .contains("missing credential_ref"));
    }

    #[test]
    fn rejects_path_traversal_in_credential_ref() {
        assert!(resolve_credential("../etc/passwd")
            .unwrap_err()
            .contains("basename"));
    }

    #[test]
    fn reads_owner_only_wifi_secret() {
        let dir = format!("/tmp/tm-wifi-secrets-{}", std::process::id());
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = format!("{dir}/home-wifi");
        let mut f = fs::File::create(&path).unwrap();
        write!(f, "test-psk\n").unwrap();
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&path, perms).unwrap();

        assert!(read_owner_only_secret(Path::new(&path)).as_deref() == Some("test-psk"));

        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&path, perms).unwrap();
        assert!(read_owner_only_secret(Path::new(&path)).is_none());

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn connect_wifi_errors_without_wpa_cli() {
        let _env = EnvRestore::set("THE_MACHINE_WPA_CLI", "/nonexistent/wpa_cli");
        let err = connect_wifi("example", "missing-ref").await.unwrap_err();
        assert!(
            err.contains("credential not found") || err.contains("wpa_cli not available"),
            "unexpected: {err}"
        );
    }

    struct EnvRestore {
        key: String,
        prev: Option<String>,
    }

    impl EnvRestore {
        fn set(key: &str, value: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self {
                key: key.to_string(),
                prev,
            }
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(&self.key, v),
                None => std::env::remove_var(&self.key),
            }
        }
    }
}
