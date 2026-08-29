//! Agent Core - the decision-making harness for The Machine.
//!
//! This is a structural implementation: it runs the session loop (wait for wake
//! -> gather context -> classify intent -> route local/cloud -> plan -> execute)
//! with a deterministic heuristic classifier and planner. Real LLM clients slot
//! in behind `classify_intent` / `plan` without changing the loop.

use common::*;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::net::UnixListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, AsyncReadExt, BufReader};
use tokio::sync::Mutex;
use tracing::{info, warn, error};

#[derive(Clone, Serialize, Deserialize, Default)]
struct Skill {
    name: String,
    #[serde(default)]
    version: u64,
    #[serde(default)]
    applies_to: Vec<String>,
    #[serde(default)]
    system_prompt: String,
    #[serde(default)]
    description: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct PlanStep {
    action: String,
    #[serde(default)]
    params: serde_json::Value,
}

struct AppState {
    status: String,
    local_only_mode: bool,
    skills: Vec<Skill>,
    interrupted: bool,
    wakes_processed: u64,
}

impl AppState {
    fn new() -> Self {
        AppState {
            status: "running".to_string(),
            local_only_mode: std::env::var("THE_MACHINE_LOCAL_ONLY_MODE")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(false),
            skills: builtin_skills(),
            interrupted: false,
            wakes_processed: 0,
        }
    }
}

fn builtin_skills() -> Vec<Skill> {
    vec![
        Skill {
            name: "intent-classification".into(),
            version: 3,
            applies_to: vec!["category:input".into()],
            system_prompt: "Classify user input into an intent and estimate complexity.".into(),
            description: "Intent classifier".into(),
        },
        Skill {
            name: "media-control".into(),
            version: 1,
            applies_to: vec!["intent:media_play".into(), "intent:media_control".into()],
            system_prompt: "Route media intents to the media_player lambda.".into(),
            description: "Media control skill".into(),
        },
    ]
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    info!("Starting Agent Core");
    let state: Arc<Mutex<AppState>> = Arc::new(Mutex::new(AppState::new()));

    // Subscribe to wakes (best-effort; the bus also hard-routes AgentWake here).
    let _ = mcp_call(
        "event.subscribe",
        serde_json::json!({ "category": "*", "pattern": "*", "subscriber": "agent-core" }),
    )
    .await;

    let socket_path = "/run/the-machine/agent-core.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path)?;
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
                    // Notifications are not answered (mirrors the bus contract).
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
    let id = msg.id;
    match msg.kind {
        MessageKind::Notification => {
            // A wake from the Event Bus. Process without replying.
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

async fn handle_request(method: String, params: Option<serde_json::Value>, state: &Arc<Mutex<AppState>>) -> McpMessage {
    let id = Uuid::new_v4();
    match method.as_str() {
        "agent.status" => {
            let s = state.lock().await;
            success_response(&id, serde_json::json!({
                "status": s.status,
                "local_model": "loaded",
                "cloud_model": if s.local_only_mode { "disabled" } else { "available" },
                "local_only_mode": s.local_only_mode,
                "wakes_processed": s.wakes_processed,
            }))
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
            success_response(&id, serde_json::json!({ "ok": true, "local_only_mode": enabled }))
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
                    })
                })
                .collect();
            success_response(&id, serde_json::json!(summaries))
        }
        _ => error_response(&id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
    }
}

/// The session loop body, invoked on each wake.
async fn process_wake(params: serde_json::Value, state: Arc<Mutex<AppState>>) {
    let wake_reason = params.clone();
    let category = wake_reason.get("category").and_then(|v| v.as_str()).unwrap_or("input").to_string();
    let pattern = wake_reason.get("pattern").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let payload = wake_reason.get("payload").cloned().unwrap_or(serde_json::Value::Null);

    // 1. Gather context (best-effort snapshots from the State Store).
    let history = mcp_call("state.get", serde_json::json!({ "path": "task.history" }))
        .await
        .and_then(|v| v.get("value").cloned())
        .unwrap_or(serde_json::Value::Array(vec![]));

    // 2. Classify intent + estimate complexity (deterministic heuristic).
    let text = payload
        .get("text")
        .and_then(|v| v.as_str())
        .or_else(|| Some(pattern.as_str()))
        .unwrap_or("")
        .to_lowercase();
    let (intent, complexity, requires_cloud) = classify(&text);

    // 3. Route (privacy gate + local-only + complexity).
    let s = state.lock().await;
    let local_only = s.local_only_mode;
    let privacy_tag = payload.get("privacy").and_then(|v| v.as_bool()).unwrap_or(false);
    drop(s);
    let routing = if privacy_tag || local_only || complexity == "low" {
        "local"
    } else if requires_cloud {
        "cloud"
    } else {
        "local"
    };

    info!(
        "wake: category={} intent={} complexity={} routing={}",
        category, intent, complexity, routing
    );

    // 4. Plan + 5. Execute.
    let plan = build_plan(&intent, &payload);
    let mut results = Vec::new();
    for step in &plan {
        if state.lock().await.interrupted {
            warn!("wake interrupted mid-plan");
            break;
        }
        let r = mcp_call(&step.action, step.params.clone()).await;
        results.push(serde_json::json!({ "action": step.action, "result": r }));
    }

    // Record the task summary.
    let mut hist = match history {
        serde_json::Value::Array(a) => a,
        _ => vec![],
    };
    hist.push(serde_json::json!({
        "intent": intent,
        "complexity": complexity,
        "routing": routing,
        "result_count": results.len(),
    }));
    // keep last 20
    if hist.len() > 20 {
        hist.drain(0..hist.len() - 20);
    }
    let _ = mcp_call(
        "state.set",
        serde_json::json!({ "path": "task.history", "value": serde_json::Value::Array(hist) }),
    )
    .await;

    let mut s = state.lock().await;
    s.wakes_processed += 1;
    s.interrupted = false;
}

/// Keyword heuristic classifier (stands in for the local model).
fn classify(text: &str) -> (String, String, bool) {
    let t = text;
    if t.contains("play") || t.contains("music") || t.contains("video") || t.contains("pause") || t.contains("stop") {
        ("media_control".into(), "low".into(), false)
    } else if t.contains("weather") || t.contains("time") || t.contains("date") || t.contains("how many") {
        ("query".into(), "low".into(), false)
    } else if t.contains("search") || t.contains("find") || t.contains("list") {
        ("search".into(), "medium".into(), false)
    } else if t.contains("build") || t.contains("create") || t.contains("register") || t.contains("make") || t.contains("generate") {
        ("lambda_register".into(), "high".into(), true)
    } else {
        ("generic".into(), "medium".into(), false)
    }
}

/// Deterministic planner: maps an intent to a sequence of MCP steps.
fn build_plan(intent: &str, payload: &serde_json::Value) -> Vec<PlanStep> {
    match intent {
        "media_control" => vec![PlanStep {
            action: "lambda.invoke".into(),
            params: serde_json::json!({
                "name": "media_player",
                "payload": { "command": "play", "query": payload.get("text").cloned().unwrap_or(serde_json::Value::Null) }
            }),
        }],
        "lambda_register" => {
            let name = payload
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("generated_fn")
                .to_string();
            vec![
                PlanStep {
                    action: "lambda.register".into(),
                    params: serde_json::json!({
                        "name": name,
                        "entrypoint": format!("/usr/bin/{}", name),
                        "capabilities": ["CAP_FS_READ"],
                    }),
                },
                PlanStep {
                    action: "state.patch".into(),
                    params: serde_json::json!({
                        "ops": [{ "path": format!("task.lambdas.{}", name), "value": { "status": "registered" } }]
                    }),
                },
            ]
        }
        "query" => vec![PlanStep {
            action: "state.set".into(),
            params: serde_json::json!({ "path": "task.last_query", "value": payload }),
        }],
        _ => vec![PlanStep {
            action: "state.set".into(),
            params: serde_json::json!({ "path": "task.last_intent", "value": intent }),
        }],
    }
}

// ---------------------------------------------------------------------------
// MCP client helper.
// ---------------------------------------------------------------------------
async fn mcp_call(method: &str, params: serde_json::Value) -> Option<serde_json::Value> {
    let path = "/run/the-machine/mcp-bus.sock";
    let stream = tokio::net::UnixStream::connect(path).await.ok()?;
    let (mut reader, mut writer) = stream.into_split();
    let req = McpMessage::request(Uuid::new_v4(), method, Some(params));
    let bytes = serde_json::to_vec(&req).ok()?;
    writer.write_all(&bytes).await.ok()?;
    writer.flush().await.ok()?;
    let mut buf = vec![0u8; 65536];
    let n = reader.read(&mut buf).await.ok()?;
    if n == 0 {
        return None;
    }
    let resp: serde_json::Value = serde_json::from_slice(&buf[..n]).ok()?;
    resp.get("result").cloned()
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
