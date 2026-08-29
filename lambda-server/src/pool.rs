//! Warm pool: pre-lease sandboxes for hot MCP routes.

use crate::sandbox::{run_persistent, persistent_kill, SandboxInput, Persistent};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

pub struct WarmPool {
    /// function name -> lease_id
    by_function: Mutex<HashMap<String, String>>,
    leases: Arc<Mutex<HashMap<String, Persistent>>>,
}

impl WarmPool {
    pub fn new(leases: Arc<Mutex<HashMap<String, Persistent>>>) -> Self {
        WarmPool {
            by_function: Mutex::new(HashMap::new()),
            leases,
        }
    }

    pub async fn warm_on_register(&self, name: &str, input: SandboxInput) -> Option<String> {
        let mut by_fn = self.by_function.lock().await;
        if by_fn.contains_key(name) {
            return by_fn.get(name).cloned();
        }
        let persistent = tokio::task::spawn_blocking(move || run_persistent(&input))
            .await
            .ok()
            .flatten()?;
        let lid = Uuid::new_v4().to_string();
        self.leases.lock().await.insert(lid.clone(), persistent);
        by_fn.insert(name.to_string(), lid.clone());
        info!("warm pool: pre-leased '{}' as {}", name, lid);
        Some(lid)
    }

    pub async fn lease_for(&self, name: &str) -> Option<String> {
        self.by_function.lock().await.get(name).cloned()
    }

    pub async fn remove_function(&self, name: &str) {
        if let Some(lid) = self.by_function.lock().await.remove(name) {
            if let Some(p) = self.leases.lock().await.remove(&lid) {
                persistent_kill(&p);
            }
        }
    }
}
