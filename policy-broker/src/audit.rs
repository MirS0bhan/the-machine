//! Audit log — append-only record of policy decisions.

use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::policy_engine::PolicyDecision;
use crate::types::AuditEntry;

pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub async fn record(
        &mut self,
        _id: &Uuid,
        capability: &str,
        path: Option<&str>,
        principal: &str,
        decision: &PolicyDecision,
    ) {
        self.entries.push(AuditEntry {
            timestamp: Utc::now().to_rfc3339(),
            method: capability.to_string(),
            request: json!({
                "path": path,
                "principal": principal,
                "reason": decision.reason,
            }),
            provenance: principal.to_string(),
            decision: decision.decision.clone(),
            correlation_id: decision.correlation_id.clone(),
        });
    }

    pub async fn record_registration(
        &mut self,
        pattern: &str,
        namespace: &str,
        registered_by: &str,
        decision: &str,
    ) {
        self.entries.push(AuditEntry {
            timestamp: Utc::now().to_rfc3339(),
            method: "mcp.intent-register".into(),
            request: json!({
                "pattern": pattern,
                "namespace": namespace,
                "registered_by": registered_by,
            }),
            provenance: registered_by.to_string(),
            decision: decision.to_string(),
            correlation_id: None,
        });
    }

    pub async fn query(&self, query: Option<Value>) -> Value {
        let mut out: Vec<Value> = self
            .entries
            .iter()
            .map(|e| serde_json::to_value(e).unwrap_or(Value::Null))
            .collect();
        if let Some(q) = query {
            if let Some(decision) = q.get("decision").and_then(|v| v.as_str()) {
                out.retain(|e| e.get("decision").and_then(|v| v.as_str()) == Some(decision));
            }
        }
        Value::Array(out)
    }
}
