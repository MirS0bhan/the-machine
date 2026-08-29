//! Persistent and in-memory storage backends for the State Store.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredValue {
    pub value: Value,
    pub revision: u64,
}

pub trait Backend: Send {
    fn get(&self, path: &str) -> Option<StoredValue>;
    fn put(&mut self, path: &str, value: Value) -> StoredValue;
    fn delete(&mut self, path: &str) -> Option<StoredValue>;
    fn list_prefix(&self, prefix: &str) -> Vec<(String, StoredValue)>;
    fn global_revision(&self) -> u64;
}

/// In-memory backend (dev / fallback).
pub struct MemoryBackend {
    data: HashMap<String, StoredValue>,
    revision: u64,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            revision: 0,
        }
    }
}

impl Backend for MemoryBackend {
    fn get(&self, path: &str) -> Option<StoredValue> {
        self.data.get(path).cloned()
    }

    fn put(&mut self, path: &str, value: Value) -> StoredValue {
        self.revision += 1;
        let sv = StoredValue {
            value,
            revision: self.revision,
        };
        self.data.insert(path.to_string(), sv.clone());
        sv
    }

    fn delete(&mut self, path: &str) -> Option<StoredValue> {
        let old = self.data.remove(path)?;
        self.revision += 1;
        Some(old)
    }

    fn list_prefix(&self, prefix: &str) -> Vec<(String, StoredValue)> {
        self.data
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn global_revision(&self) -> u64 {
        self.revision
    }
}

/// Sled embedded KV backend (persistent, pure Rust).
pub struct SledBackend {
    db: sled::Db,
    revision: u64,
}

const META_REVISION: &[u8] = b"__meta__/revision";

impl SledBackend {
    pub fn open(path: &str) -> anyhow::Result<Self> {
        std::fs::create_dir_all(path)?;
        let db = sled::open(path)?;
        let revision = db
            .get(META_REVISION)?
            .map(|b| {
                String::from_utf8_lossy(&b)
                    .parse()
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        Ok(Self { db, revision })
    }

    fn persist_revision(&mut self) {
        let _ = self.db.insert(META_REVISION, self.revision.to_string().as_bytes());
        let _ = self.db.flush();
    }
}

impl Backend for SledBackend {
    fn get(&self, path: &str) -> Option<StoredValue> {
        let raw = self.db.get(path.as_bytes()).ok()??;
        serde_json::from_slice(&raw).ok()
    }

    fn put(&mut self, path: &str, value: Value) -> StoredValue {
        self.revision += 1;
        let sv = StoredValue {
            value,
            revision: self.revision,
        };
        if let Ok(bytes) = serde_json::to_vec(&sv) {
            let _ = self.db.insert(path.as_bytes(), bytes);
            let _ = self.db.flush();
            self.persist_revision();
        }
        sv
    }

    fn delete(&mut self, path: &str) -> Option<StoredValue> {
        let old = self.get(path)?;
        self.revision += 1;
        let _ = self.db.remove(path.as_bytes());
        let _ = self.db.flush();
        self.persist_revision();
        Some(old)
    }

    fn list_prefix(&self, prefix: &str) -> Vec<(String, StoredValue)> {
        let mut out = Vec::new();
        for item in self.db.scan_prefix(prefix.as_bytes()) {
            if let Ok((k, v)) = item {
                if let Ok(s) = std::str::from_utf8(&k) {
                    if s.starts_with("__meta__") {
                        continue;
                    }
                    if let Ok(sv) = serde_json::from_slice::<StoredValue>(&v) {
                        out.push((s.to_string(), sv));
                    }
                }
            }
        }
        out
    }

    fn global_revision(&self) -> u64 {
        self.revision
    }
}

pub fn open_backend() -> Box<dyn Backend> {
    let backend = std::env::var("STATE_STORE_BACKEND").unwrap_or_else(|_| "auto".into());
    let path = std::env::var("STATE_STORE_PATH").unwrap_or_else(|_| {
        std::env::var("THE_MACHINE_DATA_DIR")
            .map(|d| format!("{}/state-store", d))
            .unwrap_or_else(|_| "/var/lib/the-machine/state-store".into())
    });

    if backend == "memory" {
        return Box::new(MemoryBackend::new());
    }

    if backend == "sled" || backend == "auto" {
        match SledBackend::open(&path) {
            Ok(b) => return Box::new(b),
            Err(e) => {
                tracing::warn!("sled open failed ({}), using memory backend", e);
            }
        }
    }

    Box::new(MemoryBackend::new())
}
