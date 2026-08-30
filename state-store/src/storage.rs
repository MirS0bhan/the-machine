//! State store with revision tracking, patch ops, and watch notifications.

use serde_json::Value;
use tokio::sync::broadcast;

use crate::backend::{open_backend, Backend, StoredValue};

#[derive(Debug, Clone)]
pub struct PatchEvent {
    pub path: String,
    pub old_value: Option<Value>,
    pub new_value: Option<Value>,
    pub revision: u64,
}

pub struct Store {
    backend: Box<dyn Backend>,
    watch_tx: broadcast::Sender<PatchEvent>,
}

impl Store {
    pub fn new() -> Self {
        let (watch_tx, _) = broadcast::channel(1024);
        Self {
            backend: open_backend(),
            watch_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PatchEvent> {
        self.watch_tx.subscribe()
    }

    pub fn get(&self, path: &str) -> Option<Value> {
        self.backend.get(path).map(|s| s.value)
    }

    pub fn get_stored(&self, path: &str) -> Option<StoredValue> {
        self.backend.get(path)
    }

    pub fn set(&mut self, path: &str, value: Option<Value>) -> u64 {
        let old = self.backend.get(path).map(|s| s.value);
        let (new_value, revision) = match value {
            Some(v) => {
                let sv = self.backend.put(path, v);
                (Some(sv.value), sv.revision)
            }
            None => {
                self.backend.delete(path);
                (None, self.backend.global_revision())
            }
        };
        let _ = self.watch_tx.send(PatchEvent {
            path: path.to_string(),
            old_value: old,
            new_value,
            revision,
        });
        revision
    }

    pub fn patch(&mut self, ops: &[Value]) -> u64 {
        let mut last_rev = self.backend.global_revision();
        for op in ops {
            let path = match op.get("path").and_then(|p| p.as_str()) {
                Some(p) => p,
                None => continue,
            };
            let op_type = op.get("op").and_then(|v| v.as_str()).unwrap_or("SET");
            match op_type.to_uppercase().as_str() {
                "SET" | "UPDATE" => {
                    if let Some(value) = op.get("value").cloned() {
                        last_rev = self.set(path, Some(value));
                    }
                }
                "INCREMENT" => {
                    let cur = self.get(path).and_then(|v| v.as_i64()).unwrap_or(0);
                    let delta = op.get("value").and_then(|v| v.as_i64()).unwrap_or(1);
                    last_rev = self.set(path, Some(Value::from(cur + delta)));
                }
                "DECREMENT" => {
                    let cur = self.get(path).and_then(|v| v.as_i64()).unwrap_or(0);
                    let delta = op.get("value").and_then(|v| v.as_i64()).unwrap_or(1);
                    last_rev = self.set(path, Some(Value::from(cur - delta)));
                }
                "TOGGLE" => {
                    let cur = self.get(path).and_then(|v| v.as_bool()).unwrap_or(false);
                    last_rev = self.set(path, Some(Value::from(!cur)));
                }
                "DELETE" | "REMOVE" => {
                    last_rev = self.set(path, None);
                }
                _ => {
                    if let Some(value) = op.get("value").cloned() {
                        last_rev = self.set(path, Some(value));
                    } else {
                        last_rev = self.set(path, None);
                    }
                }
            }
        }
        last_rev
    }

    pub fn list_prefix(&self, prefix: &str) -> Vec<(String, StoredValue)> {
        self.backend.list_prefix(prefix)
    }

    pub fn revision(&self) -> u64 {
        self.backend.global_revision()
    }
}
