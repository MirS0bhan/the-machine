//! Policy engine — port of `policy_broker/interpreter.py`.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::types::*;

/// Result of a policy check (internal).
pub struct PolicyDecision {
    pub decision: String,
    pub reason: String,
    pub correlation_id: Option<String>,
}

struct RateBucket {
    timestamps: Vec<f64>,
}

/// Rule interpreter with deny-by-default semantics.
pub struct PolicyEngine {
    policies: HashMap<String, PolicyDoc>,
    rate_limits: HashMap<String, RateBucket>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            policies: HashMap::new(),
            rate_limits: HashMap::new(),
        };
        engine.register_doc(default_policies(), "default");
        engine
    }

    pub fn register_doc(&mut self, doc: PolicyDoc, key: &str) {
        self.policies.insert(key.to_string(), doc);
    }

    pub async fn register(&mut self, policy: Option<serde_json::Value>) -> Result<(), String> {
        let policy = policy.ok_or("missing policy")?;
        let doc: PolicyDoc =
            serde_json::from_value(policy).map_err(|e| format!("invalid policy: {}", e))?;
        self.register_doc(doc, "custom");
        Ok(())
    }

    pub fn check_request(&mut self, req: &CheckRequest) -> CheckResponse {
        let capability = &req.capability;
        let path = req.path.as_deref();
        let principal = req.principal.as_deref().unwrap_or("unknown");

        if self.detect_anomaly(capability, path, principal) {
            return CheckResponse {
                decision: "DENY".into(),
                correlation_id: None,
                message: Some("Anomaly detected".into()),
            };
        }

        let rules: Vec<Rule> = self
            .policies
            .values()
            .flat_map(|p| p.rules.clone())
            .collect();

        for rule in &rules {
            if self.match_rule(rule, capability, path) {
                if !self.check_rate_limit(rule, principal) {
                    return CheckResponse {
                        decision: "DENY".into(),
                        correlation_id: None,
                        message: Some("Rate limit exceeded".into()),
                    };
                }
                let correlation_id = if rule.decision == "CONFIRM" || rule.decision == "HOLD" {
                    Some(format!("{}:{}:{}", capability, principal, now_secs()))
                } else {
                    None
                };
                return CheckResponse {
                    decision: rule.decision.clone(),
                    correlation_id,
                    message: None,
                };
            }
        }

        CheckResponse {
            decision: "DENY".into(),
            correlation_id: None,
            message: None,
        }
    }

    fn match_rule(&self, rule: &Rule, capability: &str, path: Option<&str>) -> bool {
        if !rule.capabilities.iter().any(|c| c == capability) {
            return false;
        }
        if rule.path != "*" {
            if let Some(p) = path {
                if !path_matches(&rule.path, p) {
                    return false;
                }
            }
        }
        true
    }

    fn check_rate_limit(&mut self, rule: &Rule, provenance: &str) -> bool {
        let Some(limit) = &rule.rate_limit else {
            return true;
        };
        let key = format!("{}:{}", rule.path, provenance);
        let now = now_secs();
        let window_start = now - limit.window as f64;
        let bucket = self.rate_limits.entry(key).or_insert(RateBucket {
            timestamps: Vec::new(),
        });
        bucket.timestamps.retain(|t| *t >= window_start);
        if bucket.timestamps.len() >= limit.count as usize {
            return false;
        }
        bucket.timestamps.push(now);
        true
    }

    fn detect_anomaly(&self, _capability: &str, _path: Option<&str>, _principal: &str) -> bool {
        false
    }
}

/// Shell-style glob matching (fnmatch subset used by policy rules).
pub fn path_matches(pattern: &str, path: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }
    pattern == path
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Default policies for boot + dev (mirrors Python `load_default_policies` + integration tests).
pub fn default_policies() -> PolicyDoc {
    PolicyDoc {
        rules: vec![
            Rule {
                path: "ui.*".into(),
                method: "*".into(),
                decision: "ALLOW".into(),
                capabilities: vec!["CAP_STATE_READ".into()],
                rate_limit: None,
            },
            Rule {
                path: "policy.*".into(),
                method: "*".into(),
                decision: "DENY".into(),
                capabilities: vec!["CAP_STATE_WRITE".into()],
                rate_limit: None,
            },
            Rule {
                path: "perm.mcp_routes.*".into(),
                method: "*".into(),
                decision: "ALLOW".into(),
                capabilities: vec!["mcp.intent-register".into()],
                rate_limit: None,
            },
            Rule {
                path: "lambda.*".into(),
                method: "*".into(),
                decision: "ALLOW".into(),
                capabilities: vec!["CAP_IPC_CALL".into()],
                rate_limit: None,
            },
            Rule {
                path: "calc.*".into(),
                method: "*".into(),
                decision: "ALLOW".into(),
                capabilities: vec!["CAP_IPC_CALL".into()],
                rate_limit: None,
            },
            Rule {
                path: "*".into(),
                method: "*".into(),
                decision: "ALLOW".into(),
                capabilities: vec![
                    "CAP_STATE_READ".into(),
                    "CAP_STATE_WRITE".into(),
                    "CAP_EVENT_PUBLISH".into(),
                    "CAP_TIMER".into(),
                    "CAP_EVENT_ADMIN".into(),
                    "CAP_IPC_CALL".into(),
                    "CAP_CLOUD_INFERENCE".into(),
                    "mcp.intent-register".into(),
                ],
                rate_limit: None,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> PolicyEngine {
        PolicyEngine {
            policies: HashMap::new(),
            rate_limits: HashMap::new(),
        }
    }

    #[test]
    fn allow_rule() {
        let mut e = engine();
        e.register_doc(
            PolicyDoc {
                rules: vec![Rule {
                    path: "lambda.*".into(),
                    method: "*".into(),
                    decision: "ALLOW".into(),
                    capabilities: vec!["CAP_IPC_CALL".into()],
                    rate_limit: None,
                }],
            },
            "test",
        );
        let resp = e.check_request(&CheckRequest {
            capability: "CAP_IPC_CALL".into(),
            path: Some("lambda.register".into()),
            principal: Some("agent-core".into()),
            method: Some("lambda.register".into()),
            request: None,
            provenance: Some("agent".into()),
        });
        assert_eq!(resp.decision, "ALLOW");
    }

    #[test]
    fn deny_rule() {
        let mut e = engine();
        e.register_doc(
            PolicyDoc {
                rules: vec![Rule {
                    path: "state.*".into(),
                    method: "*".into(),
                    decision: "DENY".into(),
                    capabilities: vec!["CAP_STATE_WRITE".into()],
                    rate_limit: None,
                }],
            },
            "test",
        );
        let resp = e.check_request(&CheckRequest {
            capability: "CAP_STATE_WRITE".into(),
            path: Some("ui.theme".into()),
            principal: Some("agent-core".into()),
            method: Some("state.set".into()),
            request: None,
            provenance: Some("agent".into()),
        });
        assert_eq!(resp.decision, "DENY");
    }

    #[test]
    fn first_match_wins() {
        let mut e = engine();
        e.register_doc(
            PolicyDoc {
                rules: vec![
                    Rule {
                        path: "lambda.*".into(),
                        method: "*".into(),
                        decision: "ALLOW".into(),
                        capabilities: vec!["CAP_IPC_CALL".into()],
                        rate_limit: None,
                    },
                    Rule {
                        path: "lambda.*".into(),
                        method: "lambda.register".into(),
                        decision: "DENY".into(),
                        capabilities: vec!["CAP_IPC_CALL".into()],
                        rate_limit: None,
                    },
                ],
            },
            "test",
        );
        let resp = e.check_request(&CheckRequest {
            capability: "CAP_IPC_CALL".into(),
            path: Some("lambda.register".into()),
            principal: Some("agent-core".into()),
            method: Some("lambda.register".into()),
            request: None,
            provenance: Some("agent".into()),
        });
        assert_eq!(resp.decision, "ALLOW");
    }

    #[test]
    fn default_deny_unknown() {
        let mut e = PolicyEngine {
            policies: HashMap::new(),
            rate_limits: HashMap::new(),
        };
        let resp = e.check_request(&CheckRequest {
            capability: "CAP_UNKNOWN".into(),
            path: Some("unknown".into()),
            principal: Some("agent-core".into()),
            method: Some("unknown.method".into()),
            request: None,
            provenance: Some("agent".into()),
        });
        assert_eq!(resp.decision, "DENY");
    }
}
