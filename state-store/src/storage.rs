//! In-memory state store with revision tracking.

use std::collections::HashMap;
use serde_json::Value;

/// Holds the UI/system state tree and tracks a monotonically increasing
/// revision counter for watch/subscribe semantics.
pub struct Store {
    data: HashMap<String, Value>,
    revision: u64,
}

impl Store {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            revision: 0,
        }
    }

    pub fn get(&self, path: &str) -> Option<Value> {
        self.data.get(path).cloned()
    }

    pub fn set(&mut self, path: &str, value: Option<Value>) -> u64 {
        self.revision += 1;
        match value {
            Some(v) => {
                self.data.insert(path.to_string(), v);
            }
            None => {
                self.data.remove(path);
            }
        }
        self.revision
    }

    pub fn patch(&mut self, ops: &[Value]) -> u64 {
        self.revision += 1;
        for op in ops {
            if let Some(path) = op.get("path").and_then(|p| p.as_str()) {
                match op.get("value").cloned() {
                    Some(v) => {
                        self.data.insert(path.to_string(), v);
                    }
                    None => {
                        self.data.remove(path);
                    }
                }
            }
        }
        self.revision
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }
}
