//! Static validation before broker approval.

use serde_json::Value;

const FORBIDDEN: &[&str] = &[
    "os.system",
    "subprocess.call",
    "subprocess.run",
    "eval(",
    "__import__('os')",
    "socket.socket",
    "ctypes",
];

pub struct ValidationResult {
    pub ok: bool,
    pub issues: Vec<String>,
}

pub fn validate_source(source: &str, language: &str) -> ValidationResult {
    let mut issues = Vec::new();
    for pat in FORBIDDEN {
        if source.contains(pat) {
            issues.push(format!("forbidden pattern: {}", pat));
        }
    }
    if language == "python" && !source.contains("#!/") && !source.contains("import") {
        issues.push("python source should be a script".into());
    }
    if source.len() > 64 * 1024 {
        issues.push("source exceeds 64KiB limit".into());
    }
    ValidationResult {
        ok: issues.is_empty(),
        issues,
    }
}

/// Entrypoint must be a single path under `THE_MACHINE_LAMBDA_DIR`.
/// Command lines (`/usr/bin/python3 /path/main.py`) and host binaries
/// (`/bin/sh`) are rejected so the sandbox bind-mount stays inside the
/// lambdas directory.
pub fn validate_entrypoint(entrypoint: &str) -> ValidationResult {
    let base = std::env::var("THE_MACHINE_LAMBDA_DIR")
        .unwrap_or_else(|_| "/var/the-machine/lambdas".to_string());
    validate_entrypoint_under(entrypoint, &base)
}

fn validate_entrypoint_under(entrypoint: &str, base: &str) -> ValidationResult {
    let mut issues = Vec::new();
    if entrypoint.is_empty() {
        issues.push("entrypoint is empty".into());
        return ValidationResult { ok: false, issues };
    }
    if entrypoint.chars().any(|c| c.is_whitespace()) {
        issues.push("entrypoint must be a single path, not a command line".into());
    }
    if entrypoint.contains("..") {
        issues.push("entrypoint must not contain '..'".into());
    }
    let base_norm = std::path::PathBuf::from(base)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(base));
    let candidate = std::path::PathBuf::from(entrypoint);
    let candidate_norm = if candidate.is_absolute() {
        candidate.clone()
    } else {
        base_norm.join(&candidate)
    };
    let under = path_is_under(&candidate_norm, &base_norm)
        || path_is_under(&candidate, std::path::Path::new(base));
    if !under {
        issues.push(format!(
            "entrypoint must be under THE_MACHINE_LAMBDA_DIR ({base})"
        ));
    }
    ValidationResult {
        ok: issues.is_empty(),
        issues,
    }
}

fn path_is_under(child: &std::path::Path, parent: &std::path::Path) -> bool {
    let child_s = child.to_string_lossy();
    let parent_s = parent.to_string_lossy();
    child_s == parent_s || child_s.starts_with(&format!("{parent_s}/"))
}

pub fn infer_schemas_from_source(source: &str) -> (Value, Value) {
    let input = if source.contains("sys.stdin") || source.contains("json.loads") {
        serde_json::json!({
            "type": "object",
            "properties": {
                "expression": { "type": "string" },
                "query": { "type": "string" }
            }
        })
    } else {
        serde_json::json!({ "type": "object" })
    };
    let output = serde_json::json!({
        "type": "object",
        "properties": { "result": { "type": "string" } }
    });
    (input, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_forbidden_patterns() {
        let r = validate_source("import os; os.system('rm -rf /')", "python");
        assert!(!r.ok);
        assert!(r.issues.iter().any(|i| i.contains("os.system")));
    }

    #[test]
    fn accepts_safe_python() {
        let src = "#!/usr/bin/env python3\nimport json\nprint(json.dumps({'ok': True}))";
        let r = validate_source(src, "python");
        assert!(r.ok, "{:?}", r.issues);
    }

    #[test]
    fn infers_schemas_for_stdin_script() {
        let (input, output) = infer_schemas_from_source("data = json.loads(sys.stdin.read())");
        assert!(input.get("properties").is_some());
        assert!(output.get("properties").is_some());
    }

    #[test]
    fn entrypoint_rejects_host_binaries_and_command_lines() {
        let base = "/var/the-machine/lambdas";
        assert!(!validate_entrypoint_under("/bin/sh", base).ok);
        assert!(
            !validate_entrypoint_under("/usr/bin/python3 /var/the-machine/lambdas/x/main.py", base)
                .ok
        );
        assert!(!validate_entrypoint_under("/tmp/evil", base).ok);
        assert!(!validate_entrypoint_under("/var/the-machine/lambdas/../etc/passwd", base).ok);
        assert!(validate_entrypoint_under("/var/the-machine/lambdas/calc_eval/main.py", base).ok);
    }
}
