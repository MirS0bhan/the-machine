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
}
