//! Method registry mapping MCP method names to handler component ids.

use std::collections::HashMap;

/// Maps method names to the component responsible for handling them.
pub struct Registry {
    routes: HashMap<String, String>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Register `method` as handled by `handler` (trusted flag reserved for future use).
    pub fn register(&mut self, method: &str, handler: &str, _trusted: bool) -> anyhow::Result<()> {
        self.routes.insert(method.to_string(), handler.to_string());
        Ok(())
    }

    /// Resolve a method name to its handler component id.
    pub fn resolve(&self, method: &str) -> Option<String> {
        self.routes.get(method).cloned()
    }
}
