//! Audit log: append-only record of policy decisions.

use serde_json::Value;
use uuid::Uuid;

use crate::policy_engine::PolicyDecision;

/// Append-only audit log (in-memory placeholder).
pub struct AuditLog {
    _entries: Vec<Value>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            _entries: Vec::new(),
        }
    }

    pub async fn record(&self, _id: &Uuid, _action: &str, _target: &str, _decision: &PolicyDecision) {
        // Placeholder: append to persistent audit store.
    }

    pub async fn query(&self, _query: Option<Value>) -> Value {
        Value::Array(vec![])
    }
}
