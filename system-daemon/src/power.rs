//! CPU power profile via `/sys/devices/system/cpu/cpu*/cpufreq/scaling_governor`.

const CPU_BASE: &str = "/sys/devices/system/cpu";

/// Map a sysfs governor name to the MCP profile vocabulary.
pub fn governor_to_profile(governor: &str) -> &'static str {
    match governor.trim() {
        "powersave" => "powersave",
        "performance" => "performance",
        _ => "balanced",
    }
}

/// Map an MCP profile to a sysfs governor name.
pub fn profile_to_governor(profile: &str) -> Result<&'static str, String> {
    match profile {
        "powersave" => Ok("powersave"),
        "performance" => Ok("performance"),
        "balanced" => Ok("schedutil"),
        other => Err(format!("unsupported profile: {other}")),
    }
}

fn cpu_governor_path(cpu: &str) -> String {
    format!("{CPU_BASE}/{cpu}/cpufreq/scaling_governor")
}

fn available_governors_path(cpu: &str) -> String {
    format!("{CPU_BASE}/{cpu}/cpufreq/scaling_available_governors")
}

fn list_cpu_dirs() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(CPU_BASE) else {
        return Vec::new();
    };
    let mut cpus: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit()))
        .collect();
    cpus.sort_by(|a, b| {
        let na: u32 = a[3..].parse().unwrap_or(0);
        let nb: u32 = b[3..].parse().unwrap_or(0);
        na.cmp(&nb)
    });
    cpus
}

fn pick_governor(cpu: &str, profile: &str) -> Result<String, String> {
    let preferred = profile_to_governor(profile)?;
    let avail_path = available_governors_path(cpu);
    let Ok(body) = std::fs::read_to_string(&avail_path) else {
        return Ok(preferred.to_string());
    };
    let available: Vec<&str> = body.split_whitespace().collect();
    if available.is_empty() {
        return Ok(preferred.to_string());
    }
    if available.contains(&preferred) {
        return Ok(preferred.to_string());
    }
    if profile == "balanced" {
        for fallback in ["ondemand", "conservative", "schedutil"] {
            if available.contains(&fallback) {
                return Ok(fallback.to_string());
            }
        }
    }
    Err(format!(
        "governor {preferred} not available on {cpu} (have: {body})"
    ))
}

/// Read the current power profile from cpu0's scaling governor.
pub fn read_power_profile() -> Option<String> {
    let path = cpu_governor_path("cpu0");
    let governor = std::fs::read_to_string(&path).ok()?;
    Some(governor_to_profile(&governor).to_string())
}

/// Apply a power profile by writing scaling governors for all CPUs.
pub fn write_power_profile(profile: &str) -> Result<(), String> {
    let cpus = list_cpu_dirs();
    if cpus.is_empty() {
        return Err("cpufreq sysfs not available on this host".into());
    }
    let governor = pick_governor(&cpus[0], profile)?;
    for cpu in &cpus {
        let gov_path = cpu_governor_path(cpu);
        if !std::path::Path::new(&gov_path).exists() {
            continue;
        }
        std::fs::write(&gov_path, format!("{governor}\n"))
            .map_err(|e| format!("failed to write {gov_path}: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn governor_mapping_round_trip() {
        assert_eq!(governor_to_profile("powersave"), "powersave");
        assert_eq!(governor_to_profile("performance"), "performance");
        assert_eq!(governor_to_profile("schedutil"), "balanced");
        assert_eq!(governor_to_profile("ondemand"), "balanced");
        assert_eq!(profile_to_governor("balanced").unwrap(), "schedutil");
    }

    #[test]
    fn rejects_unknown_profile() {
        assert!(profile_to_governor("turbo").is_err());
    }

    #[test]
    fn read_falls_back_when_sysfs_missing() {
        // CI VMs often lack cpufreq; ensure we do not panic.
        let _ = read_power_profile();
    }
}
