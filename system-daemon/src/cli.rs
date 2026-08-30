//! Shared subprocess helpers for host CLI tools (`pactl`, `wpa_cli`, `ip`).

use std::process::Command;

/// Run a command synchronously and return stdout on success.
pub fn run_sync(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("{program} not available: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{program} failed: {stderr}"));
    }
    Ok(stdout)
}
