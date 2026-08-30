//! Policy Broker - Capability Enforcement, Confirmation, Audit Log
//!
//! Rust implementation of the Python `policy_broker/` rule engine (Phase 3).

use common::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};

mod audit;
mod confirmation;
mod confirmation_ui;
mod policy_engine;
mod types;

use policy_engine::PolicyEngine;
use types::CheckRequest;

#[derive(Clone)]
struct AppState {
    policy_engine: Arc<Mutex<PolicyEngine>>,
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
        policy_engine: Arc::new(Mutex::new(PolicyEngine::new())),
        audit_log: Arc::new(Mutex::new(audit::AuditLog::new())),
        confirmation: Arc::new(Mutex::new(confirmation::ConfirmationDaemon::new())),
        tokens: Arc::new(RwLock::new(HashMap::new())),
    };

    {
        let confirmation = state.confirmation.clone();
        tokio::spawn(async move {
            confirmation_ui::run_confirmation_ui_loop(confirmation).await;
        });
    }

    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let socket_path = format!("{}/policy-broker.sock", socket_dir);
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let _ = tokio::fs::remove_file(&socket_path).await;
    let listener = UnixListener::bind(&socket_path)?;
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
                    if !response.is_empty() {
                        if let Err(e) = writer.write_all(response.as_bytes()).await {
                            error!("Write error: {}", e);
                            break;
                        }
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
                handle_request(method, msg.params, &msg.id, state).await
            } else {
                error_response(&msg.id, "E_INVALID_REQUEST", "Missing method")
            }
        }
        _ => error_response(&msg.id, "E_INVALID_REQUEST", "Only requests supported"),
    };

    Ok(serde_json::to_string(&response)? + "\n")
}

async fn handle_request(
    method: String,
    params: Option<serde_json::Value>,
    req_id: &Uuid,
    state: &AppState,
) -> McpMessage {
    match method.as_str() {
        "policy.check" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let check_req: CheckRequest = if params.get("capability").is_some() {
                serde_json::from_value(params.clone()).unwrap_or_else(|_| CheckRequest {
                    capability: String::new(),
                    path: None,
                    principal: None,
                    method: None,
                    request: None,
                    provenance: None,
                })
            } else {
                let m = params
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let capability = infer_capability(m);
                CheckRequest {
                    capability,
                    path: params
                        .get("request")
                        .and_then(|r| r.get("path"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| Some(m.to_string())),
                    principal: params
                        .get("provenance")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .or_else(|| {
                            params
                                .get("request")
                                .and_then(|r| r.get("principal"))
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        }),
                    method: Some(m.to_string()),
                    request: params.get("request").cloned(),
                    provenance: params
                        .get("provenance")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                }
            };

            let mut engine = state.policy_engine.lock().await;
            let resp = engine.check_request(&check_req);
            let decision = policy_engine::PolicyDecision {
                decision: resp.decision.clone(),
                reason: resp.message.clone().unwrap_or_default(),
                correlation_id: resp.correlation_id.clone(),
            };
            drop(engine);

            state
                .audit_log
                .lock()
                .await
                .record(
                    req_id,
                    &check_req.capability,
                    check_req.path.as_deref(),
                    check_req.principal.as_deref().unwrap_or("unknown"),
                    &decision,
                )
                .await;

            if resp.decision == "CONFIRM" || resp.decision == "HOLD" {
                if let Some(ref cid) = resp.correlation_id {
                    state.confirmation.lock().await.register_pending(
                        cid,
                        &check_req.capability,
                        check_req.path.as_deref(),
                        check_req.principal.as_deref().unwrap_or("unknown"),
                    );
                }
            }

            if resp.decision == "ALLOW" {
                let m = check_req.method.as_deref().unwrap_or(&check_req.capability);
                let token = shared_verifier().issue_token(
                    GrantScope {
                        method: m.to_string(),
                        request_hash: format!("{:x}", params.to_string().len()),
                        requester_identity: check_req
                            .principal
                            .clone()
                            .unwrap_or_else(|| "unknown".into()),
                    },
                    300,
                );
                state.tokens.write().await.insert(token.token_id, token.clone());
                success_response(
                    req_id,
                    serde_json::json!({
                        "decision": resp.decision,
                        "token": serde_json::to_string(&token).unwrap(),
                        "correlation_id": resp.correlation_id,
                    }),
                )
            } else {
                success_response(
                    req_id,
                    serde_json::json!({
                        "decision": resp.decision,
                        "message": resp.message,
                        "correlation_id": resp.correlation_id,
                    }),
                )
            }
        }
        "policy.register" => {
            let policy = params.and_then(|p| p.get("policy").cloned().or(Some(p)));
            match state.policy_engine.lock().await.register(policy).await {
                Ok(_) => success_response(req_id, serde_json::json!({ "ok": true })),
                Err(e) => error_response(req_id, "E_INVALID_POLICY", &e),
            }
        }
        "policy.audit_query" | "policy.audit" => {
            let query = params.and_then(|p| p.get("query").cloned().or(Some(p)));
            let entries = state.audit_log.lock().await.query(query).await;
            success_response(req_id, serde_json::json!({ "entries": entries }))
        }
        "policy.confirm_result" => {
            let cid = params
                .as_ref()
                .and_then(|p| p.get("correlation_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some(pending) = state.confirmation.lock().await.get_status(cid) {
                success_response(
                    req_id,
                    serde_json::json!({
                        "status": pending.status,
                        "correlation_id": cid,
                    }),
                )
            } else {
                error_response(req_id, "E_NOT_FOUND", "unknown correlation_id")
            }
        }
        "policy.confirm" => {
            let cid = params
                .as_ref()
                .and_then(|p| p.get("correlation_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let approved = params
                .as_ref()
                .and_then(|p| p.get("approved"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if let Some(resp) = state.confirmation.lock().await.resolve(cid, approved) {
                success_response(req_id, serde_json::to_value(resp).unwrap_or_default())
            } else {
                error_response(req_id, "E_NOT_FOUND", "unknown correlation_id")
            }
        }
        "policy.validate_register" => {
            // Internal: validate MCP intent/event route registration (mcp-bus-spec §3).
            let p = params.unwrap_or(serde_json::Value::Null);
            let pattern = p.get("pattern").and_then(|v| v.as_str()).unwrap_or("");
            let registered_by = p
                .get("registered_by")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let check_req = CheckRequest {
                capability: "mcp.intent-register".into(),
                path: Some(pattern.to_string()),
                principal: Some(registered_by.to_string()),
                method: Some("_bus.register".into()),
                request: Some(p.clone()),
                provenance: Some(registered_by.to_string()),
            };
            let mut engine = state.policy_engine.lock().await;
            let resp = engine.check_request(&check_req);
            let decision = resp.decision.clone();
            drop(engine);
            state
                .audit_log
                .lock()
                .await
                .record_registration(
                    pattern,
                    p.get("namespace")
                        .and_then(|v| v.as_str())
                        .unwrap_or("mcp-intent"),
                    registered_by,
                    &decision,
                )
                .await;
            success_response(
                req_id,
                serde_json::json!({
                    "allowed": decision == "ALLOW",
                    "decision": decision,
                }),
            )
        }
        "policy.revoke_token" => {
            if let Some(token_id_str) = params
                .as_ref()
                .and_then(|p| p.get("token_id"))
                .and_then(|v| v.as_str())
            {
                if let Ok(token_id) = Uuid::parse_str(token_id_str) {
                    state.tokens.write().await.remove(&token_id);
                    return success_response(req_id, serde_json::json!({}));
                }
            }
            error_response(req_id, "E_INVALID_REQUEST", "Invalid token_id")
        }
        "policy.hold_status" => {
            let count = state.confirmation.lock().await.pending_count();
            success_response(
                req_id,
                serde_json::json!({
                    "status": if count > 0 { "Pending" } else { "Clear" },
                    "pending_confirmations": count,
                    "reason": if count > 0 { "Awaiting confirmation" } else { "none" },
                }),
            )
        }
        "policy.confirmation.pending" => {
            let list = state.confirmation.lock().await.list_pending();
            success_response(req_id, serde_json::json!({ "pending": list }))
        }
        _ => error_response(req_id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
    }
}

/// Map an MCP method name to the capability checked by the broker middleware.
pub fn infer_capability(method: &str) -> String {
    if method.starts_with("state.") {
        if method == "state.get" || method == "state.watch" || method == "state.list" {
            "CAP_STATE_READ".into()
        } else {
            "CAP_STATE_WRITE".into()
        }
    } else if method.starts_with("event.") || method.starts_with("bus.") {
        if method.contains("schedule") || method == "event.cancel" {
            "CAP_TIMER".into()
        } else if method.contains("register") || method.contains("subscribe") {
            "CAP_EVENT_ADMIN".into()
        } else {
            "CAP_EVENT_PUBLISH".into()
        }
    } else if method.starts_with("lambda.") {
        "CAP_IPC_CALL".into()
    } else if method == "_bus.register" {
        "mcp.intent-register".into()
    } else {
        "CAP_IPC_CALL".into()
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
