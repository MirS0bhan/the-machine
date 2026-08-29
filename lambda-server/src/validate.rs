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
