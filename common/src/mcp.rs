use serde::{Deserialize, Serialize};
use uuid::Uuid;
use super::error::McpError;

/// MCP message structure
#[derive(Debug, Serialize, Deserialize)]
pub struct McpMessage {
    pub id: Uuid,
    #[serde(default)]
    pub stream_id: u64,
    #[serde(default)]
    pub kind: MessageKind,
    pub method: Option<String>,
    #[serde(default)]
    pub params: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<McpError>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub enum MessageKind {
    #[default]
    Request,
    Response,
    Notification,
    Stream,
}

impl McpMessage {
    pub fn request(id: Uuid, method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            id,
            stream_id: 0,
            kind: MessageKind::Request,
            method: Some(method.into()),
            params,
            result: None,
            error: None,
        }
    }

    pub fn response(id: Uuid, result: serde_json::Value) -> Self {
        Self {
            id,
            stream_id: 0,
            kind: MessageKind::Response,
            method: None,
            params: None,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Uuid, error: McpError) -> Self {
        Self {
            id,
            stream_id: 0,
            kind: MessageKind::Response,
            method: None,
            params: None,
            result: None,
            error: Some(error),
        }
    }
}
