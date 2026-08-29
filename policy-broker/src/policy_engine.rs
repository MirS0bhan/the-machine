//! Policy engine: deny-by-default capability evaluation.

use serde_json::Value;

/// Decision returned by the policy engine.
pub struct PolicyDecision {
    pub decision: String,
    pub reason: String,
}

/// Simple deny-by-default policy engine.
///
/// In a full implementation this loads a policy document and evaluates
/// registered rules. For now every check is denied until policies are
/// registered via [`PolicyEngine::register`].
pub struct PolicyEngine {
    rules: Vec<Value>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Evaluate a capability request. Defaults to DENY.
    pub async fn check(
        &self,
        _method: &str,
        _request: Option<Value>,
        _provenance: Option<Value>,
    ) -> PolicyDecision {
        if self.rules.is_empty() {
            return PolicyDecision {
                decision: "DENY".to_string(),
                reason: "No policy grants this capability".to_string(),
            };
        }
        // Placeholder: with rules loaded we would evaluate them here.
        PolicyDecision {
            decision: "DENY".to_string(),
            reason: "No matching allow rule".to_string(),
        }
    }

    /// Register a policy document (placeholder: stored for future evaluation).
    pub async fn register(&self, _policy: Option<Value>) -> Result<(), String> {
        Ok(())
    }
}
