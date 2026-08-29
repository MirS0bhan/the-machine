use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::debug;

pub struct Store {
    data: DashMap<String, Value>,
    pub(crate) revision: std::sync::atomic::AtomicU64,
    // For subscriptions: path prefix -> sender
    // Using broadcast channel for now; will refine
    pub(crate) watchers: Arc<DashMap<String, broadcast::Sender<Value>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            data: DashMap::new(),
            revision: std::sync::atomic::AtomicU64::new(0),
            watchers: Arc::new(DashMap::new()),
        }
    }

    pub fn get(&self, path: &str) -> Option<Value> {
        self.data.get(path).map(|entry| entry.value().clone())
    }

    pub fn set(&self, path: &str, value: Value) {
        let old = self.data.insert(path.to_string(), value.clone());
        let rev = self.revision.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        debug!("State set: {} = {} (rev {})", path, value, rev);
        // Notify watchers
        for entry in self.watchers.iter() {
            if path.starts_with(entry.key()) {
                let _ = entry.value().send(value.clone());
            }
        }
    }

    pub fn subscribe(&self, prefix: &str) -> broadcast::Receiver<Value> {
        let (tx, rx) = broadcast::channel(1024);
        self.watchers.insert(prefix.to_string(), tx);
        rx
    }

    pub fn current_revision(&self) -> u64 {
        self.revision.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}
