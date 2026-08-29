//! Confirmation daemon — tracks CONFIRM/HOLD decisions pending human approval.

use std::collections::HashMap;

use crate::types::CheckResponse;

#[derive(Debug, Clone)]
pub struct PendingConfirmation {
    pub correlation_id: String,
    pub capability: String,
    pub path: Option<String>,
    pub principal: String,
    pub status: String, // Pending | Approved | Denied
}

pub struct ConfirmationDaemon {
    pending: HashMap<String, PendingConfirmation>,
}

impl ConfirmationDaemon {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    pub fn register_pending(
        &mut self,
        correlation_id: &str,
        capability: &str,
        path: Option<&str>,
        principal: &str,
    ) {
        self.pending.insert(
            correlation_id.to_string(),
            PendingConfirmation {
                correlation_id: correlation_id.to_string(),
                capability: capability.to_string(),
                path: path.map(|s| s.to_string()),
                principal: principal.to_string(),
                status: "Pending".into(),
            },
        );
    }

    pub fn resolve(&mut self, correlation_id: &str, approved: bool) -> Option<CheckResponse> {
        let entry = self.pending.get_mut(correlation_id)?;
        entry.status = if approved {
            "Approved".into()
        } else {
            "Denied".into()
        };
        Some(CheckResponse {
            decision: if approved { "ALLOW".into() } else { "DENY".into() },
            correlation_id: Some(correlation_id.to_string()),
            message: None,
        })
    }

    pub fn get_status(&self, correlation_id: &str) -> Option<&PendingConfirmation> {
        self.pending.get(correlation_id)
    }
}
