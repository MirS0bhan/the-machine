//! Policy data types — ported from `policy_broker/models.py`.

use serde::{Deserialize, Serialize};

pub type DecisionType = String; // ALLOW | DENY | CONFIRM | HOLD

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub count: u64,
    pub window: u64, // seconds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub path: String,
    #[serde(default)]
    pub method: String,
    pub decision: DecisionType,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDoc {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRequest {
    pub capability: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub principal: Option<String>,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub request: Option<serde_json::Value>,
    #[serde(default)]
    pub provenance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResponse {
    pub decision: DecisionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub method: String,
    pub request: serde_json::Value,
    pub provenance: String,
    pub decision: DecisionType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}
