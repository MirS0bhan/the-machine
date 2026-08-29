//! Policy Broker - Capability Enforcement, Confirmation, Audit Log

use common::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, RwLock};
use tracing::{info, warn, error};

mod policy_engine;
mod audit;
mod confirmation;

#[derive(Clone)]
struct AppState {
    policy_engine: Arc<Mutex<policy_engine::PolicyEngine>>,
    audit_log: Arc<Mutex<audit::AuditLog>>,
    confirmation: Arc<Mutex<confirmation::ConfirmationDaemon>>,
    tokens: Arc<RwLock<HashMap<Uuid, GrantToken>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    info!("Starting Policy Broker");

    let state = AppState {
        policy_engine: Arc::new(Mutex::new(policy_engine::PolicyEngine::new())),
        audit_log: Arc::new(Mutex::new(audit::AuditLog::new())),
        confirmation: Arc::new(Mutex::new(confirmation::ConfirmationDaemon::new())),
        tokens: Arc::new(RwLock::new(HashMap::new())),
    };

    let socket_path = "/run/the-machine/policy-broker.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;
    info!("Policy Broker listening on {}", socket_path);

    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            handle_connection(stream, state).await;
        });
    }
}

async fn handle_connection(stream: tokio::net::UnixStream, state: AppState) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                if let Ok(response) = process_message(&line, &state).await {
                    if let Err(e) = writer.write_all(response.as_bytes()).await {
                        error!("Write error: {}", e);
                        break;
                    }
                }
            }
            Err(e) => {
                error!("Read error: {}", e);
                break;
            }
        }
    }
}

async fn process_message(line: &str, state: &AppState) -> anyhow::Result<String> {
    let msg: McpMessage = serde_json::from_str(line.trim())?;
    
    let response = match msg.kind {
        MessageKind::Request => {
            if let Some(method) = msg.method {
                handle_request(method, msg.params, state).await
            } else {
                error_response(&msg.id, "E_INVALID_REQUEST", "Missing method")
            }
        }
        _ => error_response(&msg.id, "E_INVALID_REQUEST", "Only requests supported"),
    };

    Ok(serde_json::to_string(&response)? + "\n")
}

async fn handle_request(method: String, params: Option<serde_json::Value>, state: &AppState) -> McpMessage {
    let id = Uuid::new_v4();
    
    match method.as_str() {
        "policy.check" => {
            if let Some(params) = params {
                let method_name = params.get("method").and_then(|v| v.as_str()).unwrap_or("");
                let request = params.get("request").cloned();
                let provenance = params.get("provenance").cloned();
                
                let result = state.policy_engine.lock().await.check(method_name, request.clone(), provenance).await;
                
                // Audit log
                state.audit_log.lock().await.record(&id, "policy.check", &method_name, &result).await;
                
                // If decision is ALLOW, issue token
                if result.decision == "ALLOW" {
                    let token = GrantToken {
                        token_id: Uuid::new_v4(),
                        issued_at: current_timestamp(),
                        expires_at: current_timestamp() + 300, // 5 min
                        scope: GrantScope {
                            method: method_name.to_string(),
                            request_hash: format!("{:x}", request.unwrap_or(serde_json::json!({})).to_string().len()),
                            requester_identity: "agent-core".to_string(), // hardcoded for now
                        },
                        signature: vec![0u8; 64], // placeholder
                    };
                    state.tokens.write().await.insert(token.token_id, token.clone());
                    
                    success_response(&id, serde_json::json!({
                        "decision": result.decision,
                        "token": serde_json::to_string(&token).unwrap(),
                        "reason": result.reason
                    }))
                } else {
                    success_response(&id, serde_json::json!({
                        "decision": result.decision,
                        "reason": result.reason
                    }))
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "policy.register" => {
            if let Some(params) = params {
                let policy_doc = params.get("policy").cloned();
                match state.policy_engine.lock().await.register(policy_doc).await {
                    Ok(_) => success_response(&id, serde_json::json!({})),
                    Err(e) => error_response(&id, "E_INVALID_POLICY", &e),
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "policy.audit_query" => {
            if let Some(params) = params {
                let query = params.get("query").cloned();
                let entries = state.audit_log.lock().await.query(query).await;
                success_response(&id, serde_json::json!({ "entries": entries }))
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "policy.revoke_token" => {
            if let Some(params) = params {
                if let Some(token_id_str) = params.get("token_id").and_then(|v| v.as_str()) {
                    if let Ok(token_id) = Uuid::parse_str(token_id_str) {
                        state.tokens.write().await.remove(&token_id);
                        success_response(&id, serde_json::json!({}))
                    } else {
                        error_response(&id, "E_INVALID_REQUEST", "Invalid token_id")
                    }
                } else {
                    error_response(&id, "E_INVALID_REQUEST", "Missing token_id")
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        "policy.hold_status" => {
            if let Some(params) = params {
                if let Some(hold_id) = params.get("hold_id").and_then(|v| v.as_str()) {
                    // Stub: return pending always
                    success_response(&id, serde_json::json!({
                        "status": "Pending",
                        "reason": "Rate limit exceeded",
                        "estimated_resolution": "2024-01-15T10:35:00Z"
                    }))
                } else {
                    error_response(&id, "E_INVALID_REQUEST", "Missing hold_id")
                }
            } else {
                error_response(&id, "E_INVALID_REQUEST", "Missing parameters")
            }
        }
        _ => error_response(&id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
    }
}

fn success_response(id: &Uuid, result: serde_json::Value) -> McpMessage {
    McpMessage {
        id: *id,
        stream_id: 0,
        kind: MessageKind::Response,
        method: None,
        params: None,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: &Uuid, code: &str, message: &str) -> McpMessage {
    McpMessage {
        id: *id,
        stream_id: 0,
        kind: MessageKind::Response,
        method: None,
        params: None,
        result: None,
        error: Some(McpError {
            code: code.to_string(),
            message: message.to_string(),
            details: None,
        }),
    }
}