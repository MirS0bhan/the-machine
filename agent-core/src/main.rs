//! Agent Core - LLM-driven session loop for The Machine.

mod chat;
mod client;
mod cloud;
mod desktop;
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
            refresh_cloud_router(state).await;
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
            // Re-read key so ISO mounts after boot are picked up without restart.
            refresh_cloud_router(state).await;
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
        "agent.cloud.reload" => {
            let loaded = refresh_cloud_router(state).await;
            let s = state.lock().await;
            success_response(
                &id,
                serde_json::json!({
                    "ok": true,
                    "available": loaded && !s.local_only_mode,
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
            let payload = params.clone().unwrap_or(serde_json::Value::Null);
            let inner = payload
                .get("payload")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let attachments = {
                let mut a = chat::attachments_from_payload(&payload);
                a.extend(chat::attachments_from_payload(&inner));
                // Anything staged by agent.chat.attach rides along with the send.
                for staged in take_staged_attachments().await {
                    if !a.contains(&staged) {
                        a.push(staged);
                    }
                }
                a
            };
            let source_mode = if chat::source_from_payload(&payload) != "text" {
                chat::source_from_payload(&payload)
            } else {
                chat::source_from_payload(&inner)
            };
            let routing_override = payload
                .get("routing")
                .or_else(|| inner.get("routing"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let state = state.clone();
            tokio::spawn(async move {
                process_wake(
                    serde_json::json!({
                        "category": "input",
                        "pattern": "chat.message",
                        "payload": {
                            "text": text,
                            "source": "chat_ui",
                            "attachments": attachments,
                            "input_mode": source_mode,
                            "routing": routing_override,
                        }
                    }),
                    state,
                )
                .await;
            });
            success_response(&id, serde_json::json!({ "ok": true, "queued": true }))
        }
        "agent.chat.voice" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let transcript = params
                .get("transcript")
                .or_else(|| params.get("text"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            // No transcript: this is the mic toggle. `listening` defaults to the
            // opposite of the stored state so one binding can serve both edges.
            if transcript.is_empty() {
                let was = load_bool("ui.mic_listening").await.unwrap_or(false);
                let listening = params
                    .get("listening")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(!was);
                let mut ops = chat::mic_ops(listening);
                ops.extend(chat::staged_attachment_ops(
                    &load_staged_attachments().await,
                ));
                let _ = mcp_call("ui.patch", serde_json::json!({ "ops": ops })).await;
                let _ = mcp_call(
                    "state.set",
                    serde_json::json!({ "path": "ui.mic_listening", "value": listening }),
                )
                .await;
                let note = if listening {
                    "Listening — dictate, then send the transcript to agent.chat.voice."
                } else {
                    "Dictation off."
                };
                let _ = mcp_call("ui.patch", planner::activity_plan(note).params).await;
                return success_response(
                    &id,
                    serde_json::json!({
                        "ok": true,
                        "listening": listening,
                        "accepts": "transcript",
                    }),
                );
            }
            // A transcript arrived: turn dictation off and route it as a voice turn.
            let _ = mcp_call(
                "ui.patch",
                serde_json::json!({ "ops": chat::mic_ops(false) }),
            )
            .await;
            let _ = mcp_call(
                "state.set",
                serde_json::json!({ "path": "ui.mic_listening", "value": false }),
            )
            .await;
            let attachments = take_staged_attachments().await;
            let state = state.clone();
            let spoken = transcript.clone();
            tokio::spawn(async move {
                process_wake(
                    serde_json::json!({
                        "category": "input",
                        "pattern": "chat.message",
                        "payload": {
                            "text": spoken,
                            "source": "chat_ui",
                            "attachments": attachments,
                            "input_mode": "voice",
                        }
                    }),
                    state,
                )
                .await;
            });
            success_response(
                &id,
                serde_json::json!({ "ok": true, "queued": true, "transcript": transcript }),
            )
        }
        "agent.chat.attach" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let mut refs = chat::attachments_from_payload(&params);
            if params
                .get("clear")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                refs.clear();
            } else {
                let mut existing = load_staged_attachments().await;
                for r in refs {
                    if !existing.contains(&r) {
                        existing.push(r);
                    }
                }
                refs = existing;
            }
            if refs.len() > MAX_STAGED_ATTACHMENTS {
                return error_response(
                    &id,
                    "E_INVALID",
                    &format!("at most {MAX_STAGED_ATTACHMENTS} attachments per message"),
                );
            }
            let _ = mcp_call(
                "state.set",
                serde_json::json!({
                    "path": "ui.staged_attachments",
                    "value": refs.clone(),
                }),
            )
            .await;
            let _ = mcp_call(
                "ui.patch",
                serde_json::json!({ "ops": chat::staged_attachment_ops(&refs) }),
            )
            .await;
            success_response(&id, serde_json::json!({ "ok": true, "attachments": refs }))
        }
        "agent.chat.history" => {
            let turns = load_chat_turns().await;
            success_response(
                &id,
                serde_json::json!({
                    "turns": turns,
                    "count": turns.len(),
                    "log": chat::render_log(&turns),
                }),
            )
        }
        "agent.chat.export" => {
            let turns = load_chat_turns().await;
            let transcript = chat::export_transcript(&turns);
            // Export lands on the clipboard so the user can paste it anywhere.
            let clip = mcp_call(
                "clipboard.set",
                serde_json::json!({ "text": transcript.clone() }),
            )
            .await;
            success_response(
                &id,
                serde_json::json!({
                    "transcript": transcript,
                    "turns": turns.len(),
                    "clipboard": clip.is_some(),
                }),
            )
        }
        "agent.chat.suggest" => {
            let turns = load_chat_turns().await;
            let items = chat::suggestions(&turns);
            let _ = mcp_call(
                "ui.patch",
                serde_json::json!({
                    "ops": [{
                        "op": "update",
                        "id": "ui.suggestions",
                        "props": { "items": items.clone(), "label": "Suggestions" }
                    }]
                }),
            )
            .await;
            success_response(&id, serde_json::json!({ "suggestions": items }))
        }
        "agent.chat.pin" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let n = params.get("n").and_then(|v| v.as_u64());
            let pinned = params
                .get("pinned")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let turns = load_chat_turns().await;
            let target = n.or_else(|| turns.last().map(|t| t.n));
            match target.and_then(|n| chat::set_pinned(&turns, n, pinned)) {
                Some(updated) => {
                    apply_plan_steps(&chat::turn_plan(&updated)).await;
                    success_response(
                        &id,
                        serde_json::json!({ "ok": true, "n": target, "pinned": pinned }),
                    )
                }
                None => error_response(&id, "E_NOT_FOUND", "no such chat turn"),
            }
        }
        "agent.chat.clear" => {
            let empty: Vec<chat::ChatTurn> = Vec::new();
            apply_plan_steps(&chat::turn_plan(&empty)).await;
            success_response(&id, serde_json::json!({ "ok": true, "turns": 0 }))
        }
        "agent.chat.undo" => {
            let turns = load_chat_turns().await;
            if turns.is_empty() {
                return error_response(&id, "E_NOT_FOUND", "no chat turn to undo");
            }
            let updated = chat::undo_last(&turns);
            apply_plan_steps(&chat::turn_plan(&updated)).await;
            success_response(
                &id,
                serde_json::json!({ "ok": true, "turns": updated.len() }),
            )
        }
        "agent.chat.edit" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let text = params
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if text.trim().is_empty() {
                return error_response(&id, "E_INVALID", "text required");
            }
            let turns = load_chat_turns().await;
            let n = params
                .get("n")
                .and_then(|v| v.as_u64())
                .or_else(|| turns.last().map(|t| t.n));
            let Some(n) = n else {
                return error_response(&id, "E_NOT_FOUND", "no chat turn to edit");
            };
            let Some(edited) = chat::edit_turn(&turns, n, &text) else {
                return error_response(&id, "E_NOT_FOUND", "no such chat turn");
            };
            apply_plan_steps(&chat::turn_plan(&edited)).await;
            let state = state.clone();
            let regen_text = text.clone();
            tokio::spawn(async move {
                regenerate_turn(state, n, &regen_text).await;
            });
            success_response(
                &id,
                serde_json::json!({ "ok": true, "n": n, "queued": true }),
            )
        }
        "agent.chat.regenerate" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let turns = load_chat_turns().await;
            let n = params
                .get("n")
                .and_then(|v| v.as_u64())
                .or_else(|| turns.last().map(|t| t.n));
            let Some(n) = n else {
                return error_response(&id, "E_NOT_FOUND", "no chat turn to regenerate");
            };
            let Some(turn) = turns.iter().find(|t| t.n == n).cloned() else {
                return error_response(&id, "E_NOT_FOUND", "no such chat turn");
            };
            if let Some(cleared) = chat::clear_reply(&turns, n) {
                apply_plan_steps(&chat::turn_plan(&cleared)).await;
            }
            let state = state.clone();
            let user_text = turn.user.clone();
            tokio::spawn(async move {
                regenerate_turn(state, n, &user_text).await;
            });
            success_response(
                &id,
                serde_json::json!({ "ok": true, "n": n, "queued": true }),
            )
        }
        "agent.tour.next" => {
            let step = load_u64("task.tour_step").await.unwrap_or(0) as usize;
            let plan = desktop::tour_plan(step);
            apply_plan_steps(&plan).await;
            success_response(
                &id,
                serde_json::json!({
                    "ok": true,
                    "step": (step % desktop::TOUR_TIPS.len()) + 1,
                    "total": desktop::TOUR_TIPS.len(),
                }),
            )
        }
        "agent.desktop.spawn" => {
            let params = params.unwrap_or(serde_json::Value::Null);
            let text = params
                .get("text")
                .or_else(|| params.get("kind"))
                .and_then(|v| v.as_str())
                .unwrap_or("button")
                .to_string();
            let seq = load_u64("task.spawn_seq").await.unwrap_or(1);
            let plan = match params.get("id").and_then(|v| v.as_str()) {
                Some(existing) if !existing.is_empty() => desktop::respawn_plan(&text, existing),
                _ => desktop::spawn_plan(&text, seq),
            };
            let results = apply_plan_steps(&plan).await;
            success_response(
                &id,
                serde_json::json!({ "ok": true, "steps": plan.len(), "failed": results.1 }),
            )
        }
        "agent.desktop.clear" => {
            let plan = desktop::clear_plan();
            let results = apply_plan_steps(&plan).await;
            success_response(
                &id,
                serde_json::json!({ "ok": results.1 == 0, "failed": results.1 }),
            )
        }
        _ => error_response(&id, "E_NOT_FOUND", &format!("Unknown method: {}", method)),
    }
}

/// Run a plan through the bus, returning `(results, failure_count)`.
async fn apply_plan_steps(plan: &[planner::PlanStep]) -> (Vec<serde_json::Value>, usize) {
    let mut results = Vec::new();
    let mut failed = 0usize;
    for step in plan {
        let r = mcp_call(&step.action, step.params.clone()).await;
        if r.is_none() {
            failed += 1;
        }
        results.push(serde_json::json!({ "action": step.action, "ok": r.is_some(), "result": r }));
    }
    (results, failed)
}

/// Re-run the reply chain for one turn and write the new answer back in place.
async fn regenerate_turn(state: Arc<Mutex<AppState>>, n: u64, user_text: &str) {
    let (local_only, _) = {
        let s = state.lock().await;
        (s.local_only_mode, s.cloud.is_some())
    };
    let trace = new_trace();
    let (reply, route) = resolve_chat_reply(&state, user_text, false, local_only, &trace).await;
    let turns = load_chat_turns().await;
    if let Some(updated) = chat::set_reply(&turns, n, &reply, &route) {
        let mut plan = chat::turn_plan(&updated);
        plan.push(chat::route_activity(&route));
        apply_plan_steps(&plan).await;
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
    let mut payload = wake_reason
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    // Desktop planners mint collision-free widget ids from `task.spawn_seq` and
    // advance the tour from `task.tour_step`; both ride along in the payload so
    // the pure planner functions stay side-effect free and unit-testable.
    let spawn_seq = load_u64("task.spawn_seq").await.unwrap_or(1);
    let tour_step = load_u64("task.tour_step").await.unwrap_or(0);
    if payload.is_null() {
        payload = serde_json::json!({});
    }
    if let Some(obj) = payload.as_object_mut() {
        obj.entry("spawn_seq")
            .or_insert(serde_json::json!(spawn_seq));
        obj.entry("tour_step")
            .or_insert(serde_json::json!(tour_step));
    }
    let payload = payload;

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
    let from_chat = pattern == "chat.message"
        || payload
            .get("source")
            .and_then(|v| v.as_str())
            .is_some_and(|s| s == "chat_ui");
    let active_skills = skills_for_wake(&skills, &category, "");
    // Boot is forced; chat wakes classify so actionable intents can escape pure chat.message.
    let classification = if let Some(intent) = intent_from_wake(&category, &pattern) {
        llm::Classification {
            intent,
            confidence: 1.0,
            complexity: "low".into(),
            routing: "local".into(),
            requires_cloud: false,
        }
    } else {
        let mut c = classify_intent(&text, &category, &active_skills).await;
        if from_chat {
            if let Some(desktop) = planner::desktop_intent_from_text(&text) {
                if c.intent == "chat.message" || c.intent == "generic" {
                    c.intent = desktop.to_string();
                    c.confidence = c.confidence.max(0.85);
                }
            }
        }
        c
    };
    let active_skills = skills_for_wake(&skills, &category, &classification.intent);

    let privacy_tag = payload
        .get("privacy")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // An explicit `routing` on the wake wins: "local" and "local_only" keep the
    // turn on-device, "cloud" opts in when a key is present. Unknown values fall
    // back to the classifier rather than silently escalating.
    let routing_request = payload
        .get("routing")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase());
    let local_only = local_only || routing_request.as_deref() == Some("local_only");
    let privacy_tag = privacy_tag || routing_request.as_deref() == Some("local_only");
    let routing = match routing_request.as_deref() {
        Some("local") | Some("local_only") => "local".to_string(),
        Some("heuristic") => "heuristic".to_string(),
        Some("cloud") if !privacy_tag && !local_only && cloud_router => "cloud".to_string(),
        _ if privacy_tag || local_only => "local".to_string(),
        _ if classification.requires_cloud && cloud_router => "cloud".to_string(),
        _ => classification.routing.clone(),
    };

    info!(
        "wake: category={} intent={} complexity={} routing={} confidence={}",
        category,
        classification.intent,
        classification.complexity,
        routing,
        classification.confidence
    );

    let prior_turns = load_chat_turns().await;
    let attachments = chat::attachments_from_payload(&payload);
    let source_mode = chat::source_from_payload(&payload);

    // `@skill` in the message pins that skill for this turn and says so in the
    // activity line, so the mention is visible UX rather than silent prompt text.
    let mut active_skills = active_skills;
    if let Some(mention) = chat::skill_mention(&text) {
        let all = state.lock().await.skills.clone();
        match skills::skill_by_mention(&all, &mention) {
            Some(skill) => {
                let name = skill.name.clone();
                active_skills.retain(|s| s.name != name);
                active_skills.insert(0, skill.clone());
                let _ = mcp_call(
                    "ui.patch",
                    planner::activity_plan(&format!("Using skill {name}")).params,
                )
                .await;
            }
            None => {
                let _ = mcp_call(
                    "ui.patch",
                    planner::activity_plan(&format!(
                        "No skill named @{mention} — answering without it"
                    ))
                    .params,
                )
                .await;
            }
        }
    }
    let active_skills = active_skills;

    let target_method = infer_target_method(&classification.intent, &text, &payload);
    if let Some(method) = &target_method {
        if let Some(resolved) = bus_resolve(method).await {
            if resolved.get("handler").is_some() {
                info!("resolved {} → {:?}", method, resolved.get("handler"));
                let routed = mcp_call(method, payload.clone()).await;
                if from_chat {
                    let ack = match &routed {
                        Some(_) => format!("Routed to {method}."),
                        None => format!(
                            "{method} did not answer — nothing was applied. \
                             The request is recorded and can be retried."
                        ),
                    };
                    let plan = record_turn(
                        &prior_turns,
                        &text,
                        &ack,
                        if routed.is_some() { "local" } else { "error" },
                        &attachments,
                        &source_mode,
                    );
                    apply_plan_steps(&plan).await;
                }
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
    let mut routing = routing;
    let mut turn_reply: Option<String> = None;
    let plan = if classification.intent == "chat.message" || classification.intent == "generic" {
        let (reply, reply_routing) =
            resolve_chat_reply(&state, &text, privacy_tag, local_only, &trace).await;
        info!("chat reply via {reply_routing}");
        routing = reply_routing;
        turn_reply = Some(reply);
        planner::desktop_actions_for_text(&text)
    } else if from_chat && !matches!(classification.intent.as_str(), "boot.greet" | "heartbeat") {
        // Chat UI + actionable intent: acknowledge in the multi-turn log, then run the plan.
        let (ack, ack_routing) = if cloud_router && !privacy_tag && !local_only {
            resolve_chat_reply(&state, &text, privacy_tag, local_only, &trace).await
        } else {
            (
                format!("On it — handling {}.", classification.intent),
                "local".into(),
            )
        };
        routing = ack_routing;
        turn_reply = Some(ack);
        resolve_action_plan(
            &state,
            &classification,
            &text,
            &payload,
            &active_skills,
            &routing,
            &trace,
            privacy_tag,
            local_only,
        )
        .await
    } else if classification.intent == "boot.greet" {
        boot_greet_with_memory(&prior_turns)
    } else if planner::uses_heuristic_plan(&classification.intent) {
        planner::build_plan_heuristic(&classification.intent, &payload, &text)
    } else {
        resolve_action_plan(
            &state,
            &classification,
            &text,
            &payload,
            &active_skills,
            &routing,
            &trace,
            privacy_tag,
            local_only,
        )
        .await
    };

    // Conversational turns are recorded first so the log shows the exchange even
    // if a follow-on desktop step fails.
    let mut plan = plan;
    if let Some(reply) = &turn_reply {
        let mut full = record_turn(
            &prior_turns,
            &text,
            reply,
            &routing,
            &attachments,
            &source_mode,
        );
        full.push(chat::route_activity(&routing));
        full.extend(plan);
        plan = full;
    }

    let mut results = Vec::new();
    let mut failures: Vec<String> = Vec::new();
    for step in &plan {
        if state.lock().await.interrupted {
            warn!("wake interrupted mid-plan");
            break;
        }
        let r = mcp_call(&step.action, step.params.clone()).await;
        if r.is_none() {
            warn!("plan step {} produced no result", step.action);
            failures.push(step.action.clone());
        }
        results.push(serde_json::json!({
            "action": step.action,
            "ok": r.is_some(),
            "result": r,
            "trace_id": trace,
        }));
    }

    // Fail soft, but never silently: surface what did not apply.
    if !failures.is_empty() {
        let note = fail_soft_message(&failures);
        let _ = mcp_call("ui.patch", planner::activity_plan(&note).params).await;
        let _ = mcp_call(
            "state.set",
            serde_json::json!({
                "path": "task.last_error",
                "value": {
                    "failed_steps": failures,
                    "intent": classification.intent,
                    "trace_id": trace,
                    "at": chrono::Utc::now().to_rfc3339(),
                },
            }),
        )
        .await;
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

/// Boot greeting that restores prior conversation instead of clobbering it.
///
/// Re-greeting an already-greeted session appends nothing new, so a second
/// `boot.system.ready` cannot duplicate the welcome or wipe history.
fn boot_greet_with_memory(prior: &[chat::ChatTurn]) -> Vec<planner::PlanStep> {
    let already_greeted = prior.iter().any(|t| t.assistant == planner::BOOT_WELCOME);
    let turns = if already_greeted {
        prior.to_vec()
    } else {
        let mut turn = chat::ChatTurn::new(
            chat::next_turn_number(prior),
            "",
            planner::BOOT_WELCOME,
            "boot",
        );
        turn.source = "system".into();
        chat::append_turn(prior, turn)
    };
    let mut plan = planner::boot_greet_chrome_plan(&chat::render_log(&turns), turns.len());
    plan.extend(chat::turn_plan(&turns));
    plan
}

/// Build the plan that records one conversational turn in structured form.
fn record_turn(
    prior: &[chat::ChatTurn],
    user_text: &str,
    reply: &str,
    route: &str,
    attachments: &[String],
    source_mode: &str,
) -> Vec<planner::PlanStep> {
    let mut turn = chat::ChatTurn::new(chat::next_turn_number(prior), user_text, reply, route);
    turn.attachments = attachments.to_vec();
    turn.source = source_mode.to_string();
    let turns = chat::append_turn(prior, turn);
    // A recorded user turn means the field's contents were consumed.
    chat::turn_plan_with(&turns, !user_text.trim().is_empty())
}

/// Human-readable, non-forged summary of what failed in a plan.
fn fail_soft_message(failures: &[String]) -> String {
    let unique: Vec<&String> = {
        let mut seen = Vec::new();
        for f in failures {
            if !seen.contains(&f) {
                seen.push(f);
            }
        }
        seen
    };
    let hint = if unique.iter().any(|f| f.starts_with("ui.")) {
        " UI may be stale — retry or ask again."
    } else if unique.iter().any(|f| {
        f.starts_with("power.")
            || f.starts_with("display.")
            || f.starts_with("net.")
            || f.starts_with("audio.")
    }) {
        " Privileged system change was not applied (policy or broker unavailable)."
    } else {
        " Nothing was applied for those steps."
    };
    format!("{} step(s) did not complete: {}.{hint}", unique.len(), {
        let names: Vec<String> = unique.iter().map(|s| s.to_string()).collect();
        names.join(", ")
    })
}

async fn load_chat_log() -> String {
    mcp_call("state.get", serde_json::json!({ "path": "task.chat_log" }))
        .await
        .and_then(|v| {
            v.get("value")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default()
}

/// Attachments staged by `agent.chat.attach` but not yet sent.
const MAX_STAGED_ATTACHMENTS: usize = 8;

async fn load_staged_attachments() -> Vec<String> {
    mcp_call(
        "state.get",
        serde_json::json!({ "path": "ui.staged_attachments" }),
    )
    .await
    .and_then(|v| v.get("value").cloned())
    .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
    .unwrap_or_default()
}

/// Read the staged attachments and clear them in one step, so a resend does not
/// silently re-attach the previous message's files.
async fn take_staged_attachments() -> Vec<String> {
    let staged = load_staged_attachments().await;
    if !staged.is_empty() {
        let _ = mcp_call(
            "state.set",
            serde_json::json!({ "path": "ui.staged_attachments", "value": [] }),
        )
        .await;
    }
    staged
}

async fn load_bool(path: &str) -> Option<bool> {
    mcp_call("state.get", serde_json::json!({ "path": path }))
        .await
        .and_then(|v| v.get("value").and_then(|x| x.as_bool()))
}

async fn load_u64(path: &str) -> Option<u64> {
    mcp_call("state.get", serde_json::json!({ "path": path }))
        .await
        .and_then(|v| v.get("value").and_then(|x| x.as_u64()))
}

/// Structured turns from `task.chat_turns`, migrating a legacy log blob when the
/// array is absent so history written before turns existed is not lost.
async fn load_chat_turns() -> Vec<chat::ChatTurn> {
    let value = mcp_call(
        "state.get",
        serde_json::json!({ "path": "task.chat_turns" }),
    )
    .await
    .and_then(|v| v.get("value").cloned())
    .unwrap_or(serde_json::Value::Null);
    let turns = chat::parse_turns(&value);
    if !turns.is_empty() {
        return turns;
    }
    chat::turns_from_legacy_log(&load_chat_log().await)
}

/// Multi-step MCP plan via cloud → localmodel → heuristic (general intents, not chat-only).
async fn resolve_action_plan(
    state: &Arc<Mutex<AppState>>,
    classification: &llm::Classification,
    text: &str,
    payload: &serde_json::Value,
    active_skills: &[skills::Skill],
    routing: &str,
    trace: &str,
    privacy_tag: bool,
    local_only: bool,
) -> Vec<planner::PlanStep> {
    if planner::uses_heuristic_plan(&classification.intent) {
        return planner::build_plan_heuristic(&classification.intent, payload, text);
    }
    let allow_cloud =
        !privacy_tag && !local_only && (routing == "cloud" || classification.requires_cloud);
    if allow_cloud {
        let _ = refresh_cloud_router(state).await;
        let cloud_plan = {
            let s = state.lock().await;
            if let Some(router) = &s.cloud {
                router
                    .plan(&classification.intent, text, payload, trace)
                    .await
            } else {
                None
            }
        };
        if let Some(plan) = cloud_plan {
            if !plan.is_empty() {
                return plan;
            }
        }
    }
    // Prefer local model for general intents even when routing says local.
    let local_plan = plan_from_model(&classification.intent, text, payload, active_skills).await;
    if !local_plan.is_empty() {
        return local_plan;
    }
    planner::build_plan_heuristic(&classification.intent, payload, text)
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
        // chat.message is classified from text so desktop/calc intents can win.
        (_, "chat.message") => None,
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

/// Prefer cloud (when key present), then localmodel.complete, else heuristic stub.
/// Used for conversational replies and chat acknowledgments of desktop actions.
async fn resolve_chat_reply(
    state: &Arc<Mutex<AppState>>,
    text: &str,
    privacy_tag: bool,
    local_only: bool,
    trace: &str,
) -> (String, String) {
    let allow_cloud = !privacy_tag && !local_only;
    if allow_cloud {
        let _ = refresh_cloud_router(state).await;
        let cloud_reply = {
            let s = state.lock().await;
            if let Some(router) = &s.cloud {
                router.complete_chat(text, trace).await
            } else {
                None
            }
        };
        if let Some(reply) = cloud_reply {
            return (reply, "cloud".into());
        }
    }
    if let Some(reply) = llm::complete_chat(text).await {
        return (reply, "local".into());
    }
    (planner::heuristic_chat_reply(text), "heuristic".into())
}

/// Reload cloud API key from env/secret file; returns true if a router is available.
async fn refresh_cloud_router(state: &Arc<Mutex<AppState>>) -> bool {
    let router = CloudRouter::from_env();
    let mut s = state.lock().await;
    s.cloud = router;
    s.cloud.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> Arc<Mutex<AppState>> {
        Arc::new(Mutex::new(AppState::new()))
    }

    async fn test_state_with(
        local_only_mode: bool,
        cloud: Option<CloudRouter>,
    ) -> Arc<Mutex<AppState>> {
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
    async fn agent_status_reports_running_and_unavailable_model() {
        let state = test_state();
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
            result.get("wakes_processed").and_then(|v| v.as_u64()),
            Some(0)
        );
    }

    #[tokio::test]
    async fn agent_status_reports_running_without_cloud() {
        let state = test_state_with(true, None).await;
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
        let state = test_state();
        let resp = handle_request("agent.cloud.status".into(), None, &state).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("result");
        assert_eq!(result.get("enabled").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            result.get("local_only_mode").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert!(result.get("key").is_some());

        let state = test_state_with(true, None).await;
        let resp = handle_request("agent.cloud.status".into(), None, &state).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("result");
        assert_eq!(result.get("enabled").and_then(|v| v.as_bool()), Some(false));
        assert_eq!(
            result.get("local_only_mode").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(result.get("key").is_some());
    }

    #[tokio::test]
    async fn agent_interrupt_sets_flag() {
        let state = test_state();
        let resp = handle_request("agent.interrupt".into(), None, &state).await;
        assert!(resp.error.is_none());
        assert_eq!(
            resp.result
                .as_ref()
                .and_then(|v| v.get("ok"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        assert!(state.lock().await.interrupted);
    }

    #[tokio::test]
    async fn agent_local_only_mode_toggles_state() {
        let state = test_state_with(false, None).await;
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
    async fn agent_skills_list_returns_builtin_skills() {
        let state = test_state();
        let resp = handle_request("agent.skills.list".into(), None, &state).await;
        assert!(resp.error.is_none());
        let skills = resp
            .result
            .and_then(|v| v.as_array().cloned())
            .expect("skills array");
        assert!(!skills.is_empty());
    }

    #[tokio::test]
    async fn agent_chat_send_queues_wake() {
        let state = test_state();
        let resp = handle_request(
            "agent.chat.send".into(),
            Some(serde_json::json!({
                "event": "press",
                "payload": { "text": "hello" }
            })),
            &state,
        )
        .await;
        assert!(resp.error.is_none());
        assert_eq!(
            resp.result
                .as_ref()
                .and_then(|v| v.get("queued"))
                .and_then(|v| v.as_bool()),
            Some(true)
        );
        let result = resp.result.expect("result");
        assert_eq!(result.get("ok").and_then(|v| v.as_bool()), Some(true));
    }

    #[tokio::test]
    async fn agent_cloud_reload_reports_key_status() {
        let state = test_state_with(false, None).await;
        let resp = handle_request("agent.cloud.reload".into(), None, &state).await;
        assert!(resp.error.is_none());
        let result = resp.result.expect("result");
        assert_eq!(result.get("ok").and_then(|v| v.as_bool()), Some(true));
        assert!(result.get("key").is_some());
    }

    #[tokio::test]
    async fn unknown_method_is_not_found() {
        let state = test_state();
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
