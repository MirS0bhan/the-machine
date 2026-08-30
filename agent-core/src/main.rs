//! Agent Core - LLM-driven session loop for The Machine.

mod client;
mod cloud;
mod llm;
mod planner;
mod secrets;
mod skills;

use client::mcp_call;
use cloud::{new_trace, CloudRouter};
use common::*;
use llm::{classify_intent, plan_from_model};
use skills::{load_skills, seed_default_skills_if_empty, skills_for_wake, Skill};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

struct AppState {
    status: String,
    local_only_mode: bool,
    skills: Vec<Skill>,
    interrupted: bool,
    wakes_processed: u64,
    cloud: Option<CloudRouter>,
}

impl AppState {
    fn new() -> Self {
        AppState {
            status: "running".to_string(),
            local_only_mode: std::env::var("THE_MACHINE_LOCAL_ONLY_MODE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            skills: skills::builtin_skills(),
            interrupted: false,
            wakes_processed: 0,
            cloud: CloudRouter::from_env(),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    info!("Starting Agent Core");
    seed_default_skills_if_empty().await;
    let mut initial = AppState::new();
    initial.skills = load_skills().await;

    let state: Arc<Mutex<AppState>> = Arc::new(Mutex::new(initial));

    let _ = mcp_call(
        "event.subscribe",
        serde_json::json!({ "category": "*", "pattern": "*", "subscriber": "agent-core" }),
    )
    .await;

    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let socket_path = format!("{}/agent-core.sock", socket_dir);
    let _ = std::fs::remove_file(&socket_path);
    let listener = UnixListener::bind(&socket_path)?;
    info!("Agent Core listening on {}", socket_path);

    loop {
        let (stream, _) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            handle_connection(stream, state).await;
        });
    }
}

async fn handle_connection(stream: tokio::net::UnixStream, state: Arc<Mutex<AppState>>) {
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

async fn process_message(line: &str, state: &Arc<Mutex<AppState>>) -> anyhow::Result<String> {
    let msg: McpMessage = serde_json::from_str(line.trim())?;
    let _id = msg.id;
    match msg.kind {
        MessageKind::Notification => {
            let params = msg.params.clone().unwrap_or(serde_json::Value::Null);
            let state = state.clone();
            tokio::spawn(async move {
                process_wake(params, state).await;
            });
            Ok(String::new())
        }
        MessageKind::Request => {
            let method = msg.method.clone().unwrap_or_default();
            let response = handle_request(method, msg.params, state).await;
            Ok(serde_json::to_string(&response)? + "\n")
        }
        _ => Ok(String::new()),
    }
}

async fn handle_request(
    method: String,
    params: Option<serde_json::Value>,
    state: &Arc<Mutex<AppState>>,
) -> McpMessage {
    let id = Uuid::new_v4();
    match method.as_str() {
        "agent.status" => {
            let s = state.lock().await;
            let model_status = mcp_call("localmodel.health", serde_json::json!({}))
                .await
                .and_then(|v| {
                    v.get("status")
                        .and_then(|x| x.as_str())
                        .map(|x| x.to_string())
                })
                .unwrap_or_else(|| "unavailable".into());
            let cloud_info = if let Some(router) = &s.cloud {
                serde_json::json!({
                    "available": true,
                    "key_source": router.key_source(),
                    "details": cloud::status(),
                })
            } else {
                serde_json::json!({
                    "available": false,
                    "details": cloud::status(),
                })
            };
            success_response(
                &id,
                serde_json::json!({
                    "status": s.status,
                    "local_model": model_status,
                    "cloud_model": if s.local_only_mode || s.cloud.is_none() { "disabled" } else { "available" },
                    "cloud": cloud_info,
                    "local_only_mode": s.local_only_mode,
                    "wakes_processed": s.wakes_processed,
                    "skills_loaded": s.skills.len(),
                }),
            )
        }
        "agent.cloud.status" => {
            let s = state.lock().await;
            success_response(
                &id,
                serde_json::json!({
                    "enabled": !s.local_only_mode && s.cloud.is_some(),
                    "local_only_mode": s.local_only_mode,
                    "key": cloud::status(),
                    "key_source": s.cloud.as_ref().map(|c| c.key_source()),
                }),
            )
        }
        "agent.interrupt" => {
            state.lock().await.interrupted = true;
            success_response(&id, serde_json::json!({ "ok": true }))
        }
        "agent.local_only_mode" => {
            let enabled = params
                .and_then(|p| p.get("enabled").and_then(|v| v.as_bool()))
                .unwrap_or(false);
            state.lock().await.local_only_mode = enabled;
            success_response(
                &id,
                serde_json::json!({ "ok": true, "local_only_mode": enabled }),
            )
        }
        "agent.skills.list" => {
            let s = state.lock().await;
            let summaries: Vec<serde_json::Value> = s
                .skills
                .iter()
                .map(|sk| {
                    serde_json::json!({
                        "name": sk.name,
                        "version": sk.version,
                        "applies_to": sk.applies_to,
                        "description": sk.description,
                    })
                })
                .collect();
            success_response(&id, serde_json::json!(summaries))
        }
        "agent.skills.reload" => {
            let skills = load_skills().await;
            state.lock().await.skills = skills;
            success_response(&id, serde_json::json!({ "ok": true }))
        }
        "agent.chat.send" => {
            let text = extract_chat_text(&params);
            let state = state.clone();
            tokio::spawn(async move {
                process_wake(
                    serde_json::json!({
                        "category": "input",
                        "pattern": "chat.message",
                        "payload": { "text": text, "source": "chat_ui" }
                    }),
                    state,
                )
                .await;
            });
            success_response(&id, serde_json::json!({ "ok": true, "queued": true }))
        }
        _ => error_response(&id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
    }
}

async fn process_wake(params: serde_json::Value, state: Arc<Mutex<AppState>>) {
    let wake_reason = params.clone();
    let category = wake_reason
        .get("category")
        .and_then(|v| v.as_str())
        .unwrap_or("input")
        .to_string();
    let pattern = wake_reason
        .get("pattern")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let payload = wake_reason
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let history = mcp_call("state.get", serde_json::json!({ "path": "task.history" }))
        .await
        .and_then(|v| v.get("value").cloned())
        .unwrap_or(serde_json::Value::Array(vec![]));

    let env_snapshot = payload
        .get("environment")
        .cloned()
        .or_else(|| wake_reason.get("environment").cloned());

    let env_snapshot = match env_snapshot {
        Some(v) => v,
        None => mcp_call(
            "state.get",
            serde_json::json!({ "path": "system.environment" }),
        )
        .await
        .and_then(|v| v.get("value").cloned())
        .unwrap_or(serde_json::Value::Null),
    };

    let text = payload
        .get("text")
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("query").and_then(|v| v.as_str()))
        .or_else(|| payload.get("summary").and_then(|v| v.as_str()))
        .or(Some(pattern.as_str()))
        .unwrap_or("")
        .to_string();

    let (skills, local_only, cloud_router) = {
        let s = state.lock().await;
        (s.skills.clone(), s.local_only_mode, s.cloud.is_some())
    };
    let active_skills = skills_for_wake(&skills, &category, "");
    let classification = if let Some(intent) = intent_from_wake(&category, &pattern) {
        llm::Classification {
            intent,
            confidence: 1.0,
            complexity: "low".into(),
            routing: "local".into(),
            requires_cloud: false,
        }
    } else {
        classify_intent(&text, &category, &active_skills).await
    };
    let active_skills = skills_for_wake(&skills, &category, &classification.intent);

    let privacy_tag = payload
        .get("privacy")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let routing = if privacy_tag || local_only {
        "local".to_string()
    } else if classification.requires_cloud && cloud_router {
        "cloud".to_string()
    } else {
        classification.routing.clone()
    };

    info!(
        "wake: category={} intent={} complexity={} routing={} confidence={}",
        category,
        classification.intent,
        classification.complexity,
        routing,
        classification.confidence
    );

    let target_method = infer_target_method(&classification.intent, &text, &payload);
    if let Some(method) = &target_method {
        if let Some(resolved) = bus_resolve(method).await {
            if resolved.get("handler").is_some() {
                info!("resolved {} → {:?}", method, resolved.get("handler"));
                let _ = mcp_call(method, payload.clone()).await;
                record_wake(
                    &state,
                    &classification.intent,
                    &classification.complexity,
                    &routing,
                    1,
                    &history,
                )
                .await;
                finish_wake(&state, env_snapshot).await;
                return;
            }
        }
    }

    let trace = new_trace();
    let plan = if planner::uses_heuristic_plan(&classification.intent) {
        planner::build_plan_heuristic(&classification.intent, &payload, &text)
    } else if routing == "cloud" {
        let cloud_plan = {
            let s = state.lock().await;
            if let Some(router) = &s.cloud {
                router
                    .plan(&classification.intent, &text, &payload, &trace)
                    .await
            } else {
                None
            }
        };
        cloud_plan.unwrap_or_else(|| {
            futures::executor::block_on(plan_from_model(
                &classification.intent,
                &text,
                &payload,
                &active_skills,
            ))
        })
    } else {
        plan_from_model(&classification.intent, &text, &payload, &active_skills).await
    };

    let mut results = Vec::new();
    for step in &plan {
        if state.lock().await.interrupted {
            warn!("wake interrupted mid-plan");
            break;
        }
        let r = mcp_call(&step.action, step.params.clone()).await;
        results.push(serde_json::json!({ "action": step.action, "result": r, "trace_id": trace }));
    }

    record_wake(
        &state,
        &classification.intent,
        &classification.complexity,
        &routing,
        results.len(),
        &history,
    )
    .await;
    finish_wake(&state, env_snapshot).await;
}

async fn finish_wake(state: &Arc<Mutex<AppState>>, env_snapshot: serde_json::Value) {
    if !env_snapshot.is_null() {
        let _ = mcp_call(
            "state.set",
            serde_json::json!({ "path": "system.environment", "value": env_snapshot }),
        )
        .await;
    }
    let mut s = state.lock().await;
    s.wakes_processed += 1;
    s.interrupted = false;
}

async fn record_wake(
    state: &Arc<Mutex<AppState>>,
    intent: &str,
    complexity: &str,
    routing: &str,
    result_count: usize,
    history: &serde_json::Value,
) {
    let mut hist = match history {
        serde_json::Value::Array(a) => a.clone(),
        _ => vec![],
    };
    hist.push(serde_json::json!({
        "intent": intent,
        "complexity": complexity,
        "routing": routing,
        "result_count": result_count,
        "at": chrono::Utc::now().to_rfc3339(),
    }));
    if hist.len() > 20 {
        let drain = hist.len() - 20;
        hist.drain(0..drain);
    }
    let _ = mcp_call(
        "state.set",
        serde_json::json!({ "path": "task.history", "value": serde_json::Value::Array(hist) }),
    )
    .await;
    let _ = state;
}

fn infer_target_method(intent: &str, text: &str, payload: &serde_json::Value) -> Option<String> {
    if let Some(m) = payload.get("method").and_then(|v| v.as_str()) {
        return Some(m.to_string());
    }
    match intent {
        "media_control" | "media.play" => Some("media_player.play".into()),
        "calculator" | "calc.eval" => Some("calc.eval".into()),
        _ => {
            if text.contains("calc") {
                Some("calc.eval".into())
            } else {
                None
            }
        }
    }
}

async fn bus_resolve(method: &str) -> Option<serde_json::Value> {
    mcp_call("bus.resolve", serde_json::json!({ "method": method })).await
}

fn intent_from_wake(category: &str, pattern: &str) -> Option<String> {
    match (category, pattern) {
        ("boot", "system.ready") => Some("boot.greet".into()),
        (_, "chat.message") => Some("chat.message".into()),
        (_, p) if p.contains('.') && !matches!(p, "system.ready" | "heartbeat") => {
            Some(p.to_string())
        }
        _ => None,
    }
}

fn extract_chat_text(params: &Option<serde_json::Value>) -> String {
    let Some(p) = params.as_ref() else {
        return String::new();
    };
    for key in ["text", "message"] {
        if let Some(s) = p.get(key).and_then(|v| v.as_str()) {
            if !s.is_empty() {
                return s.to_string();
            }
        }
        if let Some(s) = p
            .get("payload")
            .and_then(|pl| pl.get(key))
            .and_then(|v| v.as_str())
        {
            if !s.is_empty() {
                return s.to_string();
            }
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_state(local_only_mode: bool, cloud: Option<CloudRouter>) -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState {
            status: "running".to_string(),
            local_only_mode,
            skills: skills::builtin_skills(),
            interrupted: false,
            wakes_processed: 7,
            cloud,
        }))
    }

    #[test]
    fn extract_chat_text_reads_nested_payload() {
        let params = Some(serde_json::json!({
            "event": "press",
            "payload": { "text": "hello from chat" }
        }));
        assert_eq!(extract_chat_text(&params), "hello from chat");
    }

    #[tokio::test]
    async fn agent_status_reports_running_without_cloud() {
        let state = test_state(true, None).await;
        let resp = handle_request("agent.status".into(), None, &state).await;
        assert!(resp.error.is_none(), "unexpected {:?}", resp.error);
        let result = resp.result.expect("result");
        assert_eq!(
            result.get("status").and_then(|v| v.as_str()),
            Some("running")
        );
        assert_eq!(
            result.get("local_model").and_then(|v| v.as_str()),
            Some("unavailable")
        );
        assert_eq!(
            result.get("cloud_model").and_then(|v| v.as_str()),
            Some("disabled")
        );
        assert_eq!(
            result.get("local_only_mode").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(
            result.get("wakes_processed").and_then(|v| v.as_u64()),
            Some(7)
        );
        assert!(
            result
                .get("skills_loaded")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                > 0
        );
    }

    #[tokio::test]
    async fn agent_cloud_status_reflects_local_only_mode() {
        let state = test_state(true, None).await;
        let resp = handle_request("agent.cloud.status".into(), None, &state).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("result");
        assert_eq!(
            result.get("enabled").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            result.get("local_only_mode").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(result.get("key").is_some());
    }

    #[tokio::test]
    async fn agent_local_only_mode_toggles_state() {
        let state = test_state(false, None).await;
        let resp = handle_request(
            "agent.local_only_mode".into(),
            Some(serde_json::json!({ "enabled": true })),
            &state,
        )
        .await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("result");
        assert_eq!(result.get("ok").and_then(|v| v.as_bool()), Some(true));
        assert_eq!(
            result.get("local_only_mode").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(state.lock().await.local_only_mode);
    }

    #[tokio::test]
    async fn unknown_method_is_not_found() {
        let state = test_state(true, None).await;
        let resp = handle_request("agent.nope".into(), None, &state).await;
        assert_eq!(
            resp.error.as_ref().map(|e| e.code.as_str()),
            Some("E_NOT_FOUND")
        );
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
