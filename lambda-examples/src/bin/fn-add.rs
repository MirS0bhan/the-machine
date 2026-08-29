//! Example Lambda function: a pure integer adder.
//!
//! Reads one JSON object per line from stdin (`{"a":N,"b":N}`) and writes one
//! JSON object per line to stdout (`{"sum":N}`). Works both as a one-shot
//! (stdin closed after one request) and as a persistent lease (reads lines
//! until EOF).

use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let a = v.get("a").and_then(|x| x.as_i64()).unwrap_or(0);
        let b = v.get("b").and_then(|x| x.as_i64()).unwrap_or(0);
        let out = serde_json::json!({ "sum": a + b });
        let mut h = stdout.lock();
        let _ = writeln!(h, "{}", out);
        let _ = h.flush();
    }
}
