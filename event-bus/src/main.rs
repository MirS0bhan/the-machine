//! Event/Scheduler Bus — reactive routing, timers & agent-wake decisions.
//!
//! Implements docs/event-bus-spec.md and docs/components/event-bus.md:
//!   - publish/route events with local-resolution-first decision logic
//!   - handler registry (handles_event) and push subscriptions
//!   - a real scheduler with @every / @hourly / @daily / standard cron
//!   - per-category Agent Core wake coalescing
//!   - stats + introspection (explain_routing, list_handlers)
//!
//! Delivery to handlers/subscribers/agent is best-effort over MCP to the
//! target component's socket at /run/the-machine/<identity>.sock.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::time::Duration;

use chrono::{Datelike, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Event {
    id: Uuid,
    category: String,
    pattern: String,
    source: String,
    payload: Value,
    timestamp: i64,
    state_revision: u64,
    requires_decision: bool,
    coalesced: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HandlerEntry {
    handler: String,
    registered_at: i64,
    priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Subscription {
    id: String,
    subscriber: String,
    category: String,
    pattern: String,
    since_revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScheduledEvent {
    id: Uuid,
    trigger_time: i64,
    category: String,
    pattern: String,
    source: String,
    payload: Value,
    requires_decision: bool,
    recurring: bool,
    interval_ms: Option<i64>,
    cron_spec: Option<String>,
    max_repetitions: Option<u64>,
    repetition_count: u64,
    scheduled_by: String,
    scheduled_at: i64,
}

impl PartialEq for ScheduledEvent {
    fn eq(&self, o: &Self) -> bool {
        self.trigger_time == o.trigger_time && self.id == o.id
    }
}
impl Eq for ScheduledEvent {}
impl PartialOrd for ScheduledEvent {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for ScheduledEvent {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.trigger_time
            .cmp(&o.trigger_time)
            .then_with(|| self.id.cmp(&o.id))
    }
}

#[derive(Debug, Default)]
struct Stats {
    events_emitted: u64,
    routed_to_handler: u64,
    routed_to_subscribers: u64,
    routed_to_agent: u64,
    dropped: u64,
    agent_wakes: u64,
    agent_wakes_coalesced: u64,
    scheduled_events: u64,
    subscriptions: u64,
}

struct Scheduler {
    events: HashMap<Uuid, ScheduledEvent>,
    heap: BinaryHeap<Reverse<ScheduledEvent>>,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            events: HashMap::new(),
            heap: BinaryHeap::new(),
        }
    }
    fn insert(&mut self, se: ScheduledEvent) {
        self.events.insert(se.id, se.clone());
        self.heap.push(Reverse(se));
    }
    fn cancel(&mut self, id: &Uuid) -> bool {
        if self.events.remove(id).is_some() {
            // Rebuild heap without the removed entry.
            let kept: Vec<_> = self.heap.drain().map(|r| r.0).collect();
            self.heap = kept
                .into_iter()
                .filter(|se| &se.id != id)
                .map(Reverse)
                .collect();
            true
        } else {
            false
        }
    }
}

struct BusState {
    handlers: HashMap<(String, String), HandlerEntry>,
    subscriptions: HashMap<String, Subscription>,
    agent_wake_pending: HashMap<String, bool>,
    agent_wake_routes: Vec<(String, String)>,
    stats: Stats,
    scheduler: Scheduler,
    start_time: i64,
}

impl BusState {
    fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            subscriptions: HashMap::new(),
            agent_wake_pending: HashMap::new(),
            agent_wake_routes: Vec::new(),
            stats: Stats::default(),
            scheduler: Scheduler::new(),
            start_time: now_ms(),
        }
    }
}

type State = std::sync::Arc<Mutex<BusState>>;

/// Pattern match: supports literal, `*` (single segment), prefix `x.*` and
/// suffix `*.x`.
fn pattern_matches(pattern: &str, candidate: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(p) = pattern.strip_suffix(".*") {
        return candidate == p || candidate.starts_with(&format!("{}.", p));
    }
    if let Some(s) = pattern.strip_prefix("*.") {
        return candidate == s || candidate.ends_with(&format!(".{}", s));
    }
    let pseg: Vec<&str> = pattern.split('.').collect();
    let cseg: Vec<&str> = candidate.split('.').collect();
    if pseg.len() != cseg.len() {
        return false;
    }
    pseg.iter()
        .zip(cseg.iter())
        .all(|(p, c)| *p == "*" || p == c)
}

/// Best-effort delivery of a notification to a component's socket.
async fn deliver(identity: &str, method: &str, event: &Event) {
    let path = format!("/run/the-machine/{}.sock", identity);
    match UnixStream::connect(&path).await {
        Ok(mut s) => {
            let msg = json!({
                "id": Uuid::new_v4(),
                "kind": "Notification",
                "method": method,
                "params": serde_json::to_value(event).unwrap(),
            });
            if let Ok(mut b) = serde_json::to_vec(&msg) {
                b.push(b'\n');
                let _ = s.write_all(&b).await;
            }
        }
        Err(e) => {
            debug!("deliver to {} failed: {}", identity, e);
        }
    }
}

#[derive(Debug, Serialize)]
struct RouteSummary {
    decision: String,
    handler: Option<String>,
    subscribers: Vec<String>,
    agent_wake: bool,
}

async fn route_event(state: State, event: Event) -> RouteSummary {
    let (handler, subs, agent_wake) = {
        let mut st = state.lock().await;
        st.stats.events_emitted += 1;

        let key = (event.category.clone(), event.pattern.clone());
        let handler = st.handlers.get(&key).map(|h| h.handler.clone());

        let subs: Vec<String> = st
            .subscriptions
            .values()
            .filter(|s| {
                s.category == event.category
                    && pattern_matches(&s.pattern, &event.pattern)
                    && event.state_revision > s.since_revision
            })
            .map(|s| s.subscriber.clone())
            .collect();

        let mut agent_wake = false;
        if event.requires_decision && handler.is_none() && subs.is_empty() {
            let pending = st
                .agent_wake_pending
                .get(&event.category)
                .copied()
                .unwrap_or(false);
            if pending {
                st.stats.agent_wakes_coalesced += 1;
            } else {
                st.agent_wake_pending
                    .insert(event.category.clone(), true);
                agent_wake = true;
                st.stats.agent_wakes += 1;
                // Reset the coalescing window after a short grace period so the
                // next genuine decision point can wake the agent again.
                let cat = event.category.clone();
                let st2 = state.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let mut s = st2.lock().await;
                    s.agent_wake_pending.insert(cat, false);
                });
                if !st
                    .agent_wake_routes
                    .iter()
                    .any(|(c, p)| c == &event.category && p == &event.pattern)
                {
                    st.agent_wake_routes
                        .push((event.category.clone(), event.pattern.clone()));
                }
            }
        }

        if handler.is_some() {
            st.stats.routed_to_handler += 1;
        }
        if !subs.is_empty() {
            st.stats.routed_to_subscribers += 1;
        }
        if agent_wake {
            st.stats.routed_to_agent += 1;
        }
        if handler.is_none() && subs.is_empty() {
            if event.requires_decision {
                if !agent_wake {
                    st.stats.dropped += 1;
                }
            } else {
                st.stats.dropped += 1;
            }
        }
        (handler, subs, agent_wake)
    };

    if let Some(h) = &handler {
        deliver(h, "event.handle", &event).await;
    }
    for s in &subs {
        deliver(s, "event.notify", &event).await;
    }
    if agent_wake {
        deliver("agent-core", "event.agent_wake", &event).await;
    }

    RouteSummary {
        decision: if handler.is_some() {
            "Handler".into()
        } else if !subs.is_empty() {
            "Subscribers".into()
        } else if agent_wake {
            "AgentWake".into()
        } else {
            "Drop".into()
        },
        handler,
        subscribers: subs,
        agent_wake,
    }
}

async fn scheduler_loop(state: State) {
    loop {
        tokio::time::sleep(Duration::from_millis(100)).await;
        let due: Vec<ScheduledEvent> = {
            let mut st = state.lock().await;
            let now = now_ms();
            let mut due = Vec::new();
            let mut remaining = Vec::new();
            while let Some(Reverse(se)) = st.scheduler.heap.pop() {
                if se.trigger_time <= now {
                    due.push(se);
                } else {
                    remaining.push(se);
                }
            }
            for r in remaining {
                st.scheduler.heap.push(Reverse(r));
            }
            for se in &due {
                st.scheduler.events.remove(&se.id);
            }
            due
        };

        for se in due {
            let event = Event {
                id: Uuid::new_v4(),
                category: se.category.clone(),
                pattern: se.pattern.clone(),
                source: se.scheduled_by.clone(),
                payload: se.payload.clone(),
                timestamp: now_ms(),
                state_revision: 0,
                requires_decision: se.requires_decision,
                coalesced: false,
            };

            if se.recurring {
                let next = if let Some(iv) = se.interval_ms {
                    se.trigger_time + iv
                } else if let Some(spec) = se.cron_spec.clone() {
                    next_cron_time(&spec, se.trigger_time + 1)
                } else {
                    se.trigger_time + 60_000
                };
                let repetition_count = se.repetition_count + 1;
                let keep = se
                    .max_repetitions
                    .map_or(true, |m| repetition_count <= m);
                if keep {
                    let new_se = ScheduledEvent {
                        trigger_time: next,
                        repetition_count,
                        ..se
                    };
                    let mut st = state.lock().await;
                    st.scheduler.insert(new_se);
                }
            }

            let s = state.clone();
            tokio::spawn(async move {
                route_event(s, event).await;
            });
        }
    }
}

// ----- cron / duration parsing -----

fn parse_duration(s: &str) -> Result<i64, String> {
    let s = s.trim();
    let split = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
    let (num, unit) = s.split_at(split);
    let n: f64 = num.parse().map_err(|_| "bad duration".to_string())?;
    let ms = match unit {
        "ms" => n,
        "s" => n * 1_000.0,
        "m" => n * 60_000.0,
        "h" => n * 3_600_000.0,
        "d" => n * 86_400_000.0,
        _ => return Err(format!("bad duration unit: {}", unit)),
    };
    Ok(ms as i64)
}

fn field_matches(field: &str, v: i64) -> bool {
    if field == "*" {
        return true;
    }
    for part in field.split(',') {
        if let Some(step) = part.strip_prefix("*/") {
            if let Ok(s) = step.parse::<i64>() {
                if s > 0 && v % s == 0 {
                    return true;
                }
                continue;
            }
        }
        if let Some((a, b)) = part.split_once('-') {
            if let (Ok(a), Ok(b)) = (a.parse::<i64>(), b.parse::<i64>()) {
                if v >= a && v <= b {
                    return true;
                }
                continue;
            }
        }
        if let Ok(n) = part.parse::<i64>() {
            if n == v {
                return true;
            }
        }
    }
    false
}

fn next_cron_time(spec: &str, after_ms: i64) -> i64 {
    let fields: Vec<&str> = spec.split_whitespace().collect();
    if fields.len() != 5 {
        return after_ms + 60_000;
    }
    let base = Utc
        .timestamp_millis_opt(after_ms)
        .single()
        .unwrap_or_else(Utc::now);
    let mut cur = base + chrono::Duration::seconds(1);
    cur = cur
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    for _ in 0..500_000 {
        let m = cur.minute() as i64;
        let h = cur.hour() as i64;
        let d = cur.day() as i64;
        let mo = cur.month() as i64;
        let wd = cur.weekday().num_days_from_sunday() as i64;
        if field_matches(fields[0], m)
            && field_matches(fields[1], h)
            && field_matches(fields[2], d)
            && field_matches(fields[3], mo)
            && field_matches(fields[4], wd)
        {
            return cur.timestamp_millis();
        }
        cur = cur + chrono::Duration::minutes(1);
    }
    after_ms + 60_000
}

fn next_hourly(now_ms: i64) -> i64 {
    let t = Utc.timestamp_millis_opt(now_ms).single().unwrap();
    let nxt = t + chrono::Duration::hours(1);
    nxt.with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
        .timestamp_millis()
}

fn next_daily(now_ms: i64) -> i64 {
    let t = Utc.timestamp_millis_opt(now_ms).single().unwrap();
    let tomorrow = t.date_naive() + chrono::Duration::days(1);
    tomorrow
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
        .timestamp_millis()
}

fn parse_when(spec: &str, now: i64) -> Result<(i64, Option<i64>), String> {
    let spec = spec.trim();
    if let Some(d) = spec.strip_prefix("@every ") {
        let ms = parse_duration(d)?;
        return Ok((now + ms, Some(ms)));
    }
    match spec {
        "@hourly" => Ok((next_hourly(now), Some(3_600_000))),
        "@daily" => Ok((next_daily(now), Some(86_400_000))),
        _ => {
            let t = next_cron_time(spec, now);
            Ok((t, None))
        }
    }
}

async fn gather_environment_snapshot() -> Value {
    let uptime = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next().map(|v| v.to_string()))
        .unwrap_or_else(|| "0".into());
    json!({
        "timestamp_ms": now_ms(),
        "uptime_secs": uptime,
        "hostname": std::env::var("HOSTNAME").unwrap_or_else(|_| "the-machine".into()),
    })
}

/// Emit periodic heartbeat events with environment snapshots to wake the agent proactively.
async fn heartbeat_loop(state: State) {
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        let environment = gather_environment_snapshot().await;
        let payload = json!({
            "kind": "heartbeat",
            "environment": environment,
        });
        let event = Event {
            id: Uuid::new_v4(),
            category: "scheduler".into(),
            pattern: "heartbeat.tick".into(),
            source: "event-bus".into(),
            payload,
            timestamp: now_ms(),
            state_revision: 0,
            requires_decision: true,
            coalesced: false,
        };
        route_event(state.clone(), event).await;
    }
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "id": id, "result": result })
}
fn err(id: Value, code: &str, message: &str) -> Value {
    json!({ "id": id, "error": { "code": code, "message": message } })
}
fn pstr(params: &Option<Value>, key: &str) -> Option<String> {
    params
        .as_ref()
        .and_then(|p| p.get(key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}
fn pbool(params: &Option<Value>, key: &str) -> bool {
    params
        .as_ref()
        .and_then(|p| p.get(key))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

async fn handle_request(value: Value, state: State) -> Value {
    let id = value.get("id").cloned().unwrap_or(Value::Null);
    let method = match value.get("method").and_then(|m| m.as_str()) {
        Some(m) => m.to_string(),
        None => return err(id, "E_INVALID_REQUEST", "missing method"),
    };
    let params = value.get("params").cloned();
    debug!("event-bus method: {}", method);

    match method.as_str() {
        "hello" => ok(id, json!({"status": "ok"})),
        "event.publish" | "event.emit" => publish(params, state, id).await,
        "event.subscribe" => subscribe(params, state, id).await,
        "event.unsubscribe" => unsubscribe(params, state, id).await,
        "event.register_handler" => register_handler(params, state, id).await,
        "event.schedule" => schedule(params, state, id).await,
        "event.cancel" => cancel(params, state, id).await,
        "event.stats" => stats(state, id).await,
        "event.explain_routing" => explain(state, params, id).await,
        "event.list_handlers" | "bus.list_handlers" => list_handlers(state, id).await,
        "event.list_agent_wakes" => list_agent_wakes(state, id).await,
        _ => err(
            id,
            "E_NOT_FOUND",
            &format!("unknown method: {}", method),
        ),
    }
}

async fn publish(params: Option<Value>, state: State, id: Value) -> Value {
    let p = params.clone().unwrap_or(Value::Null);
    let category = match pstr(&params, "category") {
        Some(c) => c,
        None => return err(id, "E_INVALID_CATEGORY", "category required"),
    };
    let pattern = pstr(&params, "pattern").unwrap_or_default();
    let payload = p.get("payload").cloned().unwrap_or(Value::Null);
    let requires_decision = pbool(&params, "requires_decision");
    let source = pstr(&params, "source").unwrap_or_else(|| "unknown".into());

    let event = Event {
        id: Uuid::new_v4(),
        category,
        pattern,
        source,
        payload,
        timestamp: now_ms(),
        state_revision: p
            .get("state_revision")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        requires_decision,
        coalesced: false,
    };

    let summary = route_event(state, event.clone()).await;
    ok(
        id,
        json!({
            "event_id": event.id,
            "decision": summary.decision,
            "handler": summary.handler,
            "subscribers": summary.subscribers,
            "agent_wake": summary.agent_wake,
        }),
    )
}

async fn subscribe(params: Option<Value>, state: State, id: Value) -> Value {
    let category = match pstr(&params, "category") {
        Some(c) => c,
        None => return err(id, "E_INVALID_CATEGORY", "category required"),
    };
    let pattern = pstr(&params, "pattern").unwrap_or_else(|| "*".into());
    let subscriber = pstr(&params, "subscriber").unwrap_or_else(|| "anonymous".into());
    let since_revision = params
        .as_ref()
        .and_then(|p| p.get("since_revision"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let sub_id = Uuid::new_v4().to_string();
    {
        let mut st = state.lock().await;
        st.subscriptions.insert(
            sub_id.clone(),
            Subscription {
                id: sub_id.clone(),
                subscriber,
                category,
                pattern,
                since_revision,
            },
        );
        st.stats.subscriptions += 1;
    }
    ok(id, json!({ "subscription_id": sub_id }))
}

async fn unsubscribe(params: Option<Value>, state: State, id: Value) -> Value {
    let sub_id = match pstr(&params, "subscription_id") {
        Some(s) => s,
        None => return err(id, "E_INVALID", "subscription_id required"),
    };
    let removed = {
        let mut st = state.lock().await;
        st.subscriptions.remove(&sub_id).is_some()
    };
    if removed {
        ok(id, json!({}))
    } else {
        err(id, "E_NOT_FOUND", "unknown subscription_id")
    }
}

async fn register_handler(params: Option<Value>, state: State, id: Value) -> Value {
    let category = match pstr(&params, "category") {
        Some(c) => c,
        None => return err(id, "E_INVALID_CATEGORY", "category required"),
    };
    let pattern = match pstr(&params, "pattern") {
        Some(p) if !p.is_empty() => p,
        _ => return err(id, "E_INVALID_PATTERN", "pattern required"),
    };
    let handler = match pstr(&params, "handler") {
        Some(h) => h,
        None => return err(id, "E_INVALID", "handler required"),
    };
    let priority = params
        .as_ref()
        .and_then(|p| p.get("priority"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    {
        let mut st = state.lock().await;
        st.handlers.insert(
            (category.clone(), pattern.clone()),
            HandlerEntry {
                handler: handler.clone(),
                registered_at: now_ms(),
                priority,
            },
        );
    }
    ok(
        id,
        json!({ "registered": true, "handler": handler, "category": category, "pattern": pattern }),
    )
}

async fn schedule(params: Option<Value>, state: State, id: Value) -> Value {
    let cron = match pstr(&params, "cron") {
        Some(c) if !c.is_empty() => c,
        _ => return err(id, "E_INVALID_CRON", "cron required"),
    };
    let (trigger_time, interval) = match parse_when(&cron, now_ms()) {
        Ok(v) => v,
        Err(e) => return err(id, "E_INVALID_CRON", &e),
    };
    let p = params.clone().unwrap_or(Value::Null);
    let category = pstr(&params, "category").unwrap_or_else(|| "external".into());
    let pattern = pstr(&params, "pattern").unwrap_or_else(|| "timer.fire".into());
    let source = pstr(&params, "scheduled_by").unwrap_or_else(|| "unknown".into());
    let payload = p.get("payload").cloned().unwrap_or(Value::Null);
    let recurring = pbool(&params, "recurring");
    let requires_decision = pbool(&params, "requires_decision");
    let max_repetitions = p
        .get("max_repetitions")
        .and_then(|v| v.as_u64());
    let cron_spec = if interval.is_none() && recurring {
        Some(cron.clone())
    } else {
        None
    };

    let se = ScheduledEvent {
        id: Uuid::new_v4(),
        trigger_time,
        category,
        pattern,
        source: source.clone(),
        payload,
        requires_decision,
        recurring,
        interval_ms: interval,
        cron_spec,
        max_repetitions,
        repetition_count: 0,
        scheduled_by: source,
        scheduled_at: now_ms(),
    };
    {
        let mut st = state.lock().await;
        st.scheduler.insert(se.clone());
        st.stats.scheduled_events += 1;
    }
    ok(
        id,
        json!({
            "event_id": se.id,
            "trigger_time": trigger_time,
            "recurring": recurring,
        }),
    )
}

async fn cancel(params: Option<Value>, state: State, id: Value) -> Value {
    let event_id = match pstr(&params, "event_id") {
        Some(s) => s,
        None => return err(id, "E_INVALID", "event_id required"),
    };
    let uuid = match Uuid::parse_str(&event_id) {
        Ok(u) => u,
        Err(_) => return err(id, "E_INVALID", "invalid event_id"),
    };
    let cancelled = {
        let mut st = state.lock().await;
        st.scheduler.cancel(&uuid)
    };
    if cancelled {
        ok(id, json!({ "cancelled": true }))
    } else {
        err(id, "E_NOT_FOUND", "unknown event_id")
    }
}

async fn stats(state: State, id: Value) -> Value {
    let st = state.lock().await;
    let s = &st.stats;
    ok(
        id,
        json!({
            "events_emitted": s.events_emitted,
            "routed_to_handler": s.routed_to_handler,
            "routed_to_subscribers": s.routed_to_subscribers,
            "routed_to_agent": s.routed_to_agent,
            "dropped": s.dropped,
            "agent_wakes": s.agent_wakes,
            "agent_wakes_coalesced": s.agent_wakes_coalesced,
            "scheduled_events": s.scheduled_events,
            "subscriptions": s.subscriptions,
            "uptime_ms": now_ms() - st.start_time,
        }),
    )
}

async fn explain(state: State, params: Option<Value>, id: Value) -> Value {
    let category = match pstr(&params, "category") {
        Some(c) => c,
        None => return err(id, "E_INVALID_CATEGORY", "category required"),
    };
    let pattern = pstr(&params, "pattern").unwrap_or_else(|| "*".into());
    let requires_decision = pbool(&params, "requires_decision");

    let st = state.lock().await;
    let key = (category.clone(), pattern.clone());
    if let Some(h) = st.handlers.get(&key) {
        return ok(
            id,
            json!({
                "decision": "Handler",
                "reason": "ExplicitHandler",
                "handler": h.handler,
            }),
        );
    }
    let sub_count = st
        .subscriptions
        .values()
        .filter(|s| {
            s.category == category
                && pattern_matches(&s.pattern, &pattern)
        })
        .count();
    if sub_count > 0 {
        return ok(
            id,
            json!({
                "decision": "Subscribers",
                "reason": "MatchingSubscribers",
                "count": sub_count,
            }),
        );
    }
    if requires_decision {
        return ok(
            id,
            json!({ "decision": "AgentWake", "reason": "RequiresDecision" }),
        );
    }
    ok(id, json!({ "decision": "Drop", "reason": "NoMatch" }))
}

async fn list_handlers(state: State, id: Value) -> Value {
    let st = state.lock().await;
    let handlers: Vec<Value> = st
        .handlers
        .iter()
        .map(|((c, p), h)| {
            json!({
                "category": c,
                "pattern": p,
                "handler": h.handler,
                "priority": h.priority,
            })
        })
        .collect();
    ok(id, json!({ "handlers": handlers }))
}

async fn list_agent_wakes(state: State, id: Value) -> Value {
    let st = state.lock().await;
    let patterns: Vec<Value> = st
        .agent_wake_routes
        .iter()
        .map(|(c, p)| json!({ "category": c, "pattern": p }))
        .collect();
    ok(id, json!({ "patterns": patterns }))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()))
        .init();

    info!("Starting Event Bus");

    let state: State = std::sync::Arc::new(Mutex::new(BusState::new()));

    // Scheduler worker.
    {
        let s = state.clone();
        tokio::spawn(async move {
            scheduler_loop(s).await;
        });
    }

    // Proactive heartbeat with environment snapshot.
    {
        let s = state.clone();
        tokio::spawn(async move {
            heartbeat_loop(s).await;
        });
    }

    let socket_dir =
        std::env::var("THE_MACHINE_SOCKET_DIR").unwrap_or_else(|_| "/run/the-machine".to_string());
    let socket_path = format!("{}/event-bus.sock", socket_dir);
    if let Some(parent) = std::path::Path::new(&socket_path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let _ = tokio::fs::remove_file(&socket_path).await;
    let listener = UnixListener::bind(&socket_path)?;
    info!("Event Bus listening on {}", socket_path);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state = state.clone();
                tokio::spawn(handle_connection(stream, state));
            }
            Err(e) => {
                error!("accept error: {}", e);
            }
        }
    }
}

async fn handle_connection(mut stream: UnixStream, state: State) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let value: Value = match serde_json::from_str(trimmed) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("invalid JSON: {}", e);
                        continue;
                    }
                };
                // Notifications are not answered.
                let is_request = value
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .map(|k| k != "Notification")
                    .unwrap_or(true);
                if !is_request {
                    continue;
                }
                let response = handle_request(value, state.clone()).await;
                if let Ok(bytes) = serde_json::to_vec(&response) {
                    let mut buf = bytes;
                    buf.push(b'\n');
                    if writer.write_all(&buf).await.is_err() {
                        break;
                    }
                }
            }
            Err(e) => {
                debug!("read error: {}", e);
                break;
            }
        }
    }
}
