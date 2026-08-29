use serde::{Deserialize, Serialize};

/// Error structure for MCP responses
#[derive(Debug, Serialize, Deserialize)]
pub struct McpError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl McpError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: serde_json::Value,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details),
        }
    }
}
