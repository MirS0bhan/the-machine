//! MCP call graph observability (in-memory spans, exportable).

use dashmap::DashMap;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone)]
pub struct Span {
    pub trace_id: String,
    pub span_id: String,
    pub method: String,
    pub handler: String,
    pub started_ms: u64,
    pub duration_ms: u64,
    pub ok: bool,
}

pub struct Telemetry {
    spans: DashMap<String, Span>,
    max_spans: usize,
}

impl Telemetry {
    pub fn new(max_spans: usize) -> Self {
        Telemetry {
            spans: DashMap::new(),
            max_spans: max_spans.max(100),
        }
    }

    pub fn record(&self, method: &str, handler: &str, duration_ms: u64, ok: bool) -> String {
        let trace_id = Uuid::new_v4().to_string();
        let span_id = Uuid::new_v4().to_string();
        self.spans.insert(
            span_id.clone(),
            Span {
                trace_id: trace_id.clone(),
                span_id: span_id.clone(),
                method: method.to_string(),
                handler: handler.to_string(),
                started_ms: now_ms(),
                duration_ms,
                ok,
            },
        );
        if self.spans.len() > self.max_spans {
            let keys: Vec<String> = self
                .spans
                .iter()
                .take(self.spans.len() - self.max_spans)
                .map(|e| e.key().clone())
                .collect();
            for k in keys {
                self.spans.remove(&k);
            }
        }
        trace_id
    }

    pub fn export(&self, limit: usize) -> Value {
        let items: Vec<Value> = self
            .spans
            .iter()
            .take(limit)
            .map(|e| {
                json!({
                    "trace_id": e.trace_id,
                    "span_id": e.span_id,
                    "method": e.method,
                    "handler": e.handler,
                    "started_ms": e.started_ms,
                    "duration_ms": e.duration_ms,
                    "ok": e.ok,
                })
            })
            .collect();
        json!({ "spans": items, "count": items.len() })
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub type SharedTelemetry = Arc<Telemetry>;
