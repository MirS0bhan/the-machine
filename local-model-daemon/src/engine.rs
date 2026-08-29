//! Local Tier-A inference: stub heuristics with optional HTTP proxy to Python llama.cpp.

use serde_json::{json, Value};

pub struct Engine {
    http_url: Option<String>,
    stub: bool,
}

impl Engine {
    pub fn new() -> Self {
        let http_url = std::env::var("LOCAL_MODEL_HTTP_URL")
            .ok()
            .or_else(|| {
                std::env::var("LOCAL_MODEL_PATH")
                    .ok()
                    .filter(|p| std::path::Path::new(p).is_file())
                    .map(|_| "http://127.0.0.1:8010".to_string())
            });
        let stub = http_url.is_none();
        Engine { http_url, stub }
    }

    pub fn health(&self) -> Value {
        json!({
            "status": if self.stub { "stub" } else { "ready" },
            "backend": if self.stub { "builtin" } else { "http" },
            "http_url": self.http_url,
        })
    }

    pub async fn complete(&self, prompt: &str, max_tokens: u32, temperature: f32) -> String {
        if let Some(url) = &self.http_url {
            if let Ok(client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(60)).build() {
                let body = json!({
                    "prompt": prompt,
                    "max_tokens": max_tokens,
                    "temperature": temperature,
                    "privacy_tags": [],
                });
                if let Ok(resp) = client
                    .post(format!("{}/mcp/localmodel.complete", url.trim_end_matches('/')))
                    .json(&body)
                    .send()
                    .await
                {
                    if let Ok(v) = resp.json::<Value>().await {
                        if let Some(t) = v.get("text").and_then(|x| x.as_str()) {
                            return t.to_string();
                        }
                    }
                }
            }
        }
        self.complete_stub(prompt)
    }

    fn complete_stub(&self, prompt: &str) -> String {
        if prompt.contains("JSON plan") || prompt.contains("\"steps\"") {
            return r#"{"intent":"synthesize","complexity":"medium","requires_cloud":false,"steps":[{"action":"state.set","params":{"path":"task.last_plan","value":"generated"}}]}"#.to_string();
        }
        format!("[local-stub] {}", &prompt[..prompt.len().min(120)])
    }

    pub async fn classify_intent(&self, text: &str, category: &str) -> (String, f64, String, String, bool) {
        if let Some(url) = &self.http_url {
            if let Ok(client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build() {
                let body = json!({ "text": text, "category": category, "privacy_tags": [] });
                if let Ok(resp) = client
                    .post(format!("{}/mcp/localmodel.classify_intent", url.trim_end_matches('/')))
                    .json(&body)
                    .send()
                    .await
                {
                    if let Ok(v) = resp.json::<Value>().await {
                        let intent = v.get("intent").and_then(|x| x.as_str()).unwrap_or("generic").to_string();
                        let confidence = v.get("confidence").and_then(|x| x.as_f64()).unwrap_or(0.8);
                        return (
                            intent.clone(),
                            confidence,
                            complexity_for(&intent),
                            routing_for(&intent),
                            requires_cloud(&intent),
                        );
                    }
                }
            }
        }
        classify_stub(text, category)
    }

    pub async fn embed(&self, text: &str) -> Vec<f32> {
        if let Some(url) = &self.http_url {
            if let Ok(client) = reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build() {
                let body = json!({ "text": text, "privacy_tags": [] });
                if let Ok(resp) = client
                    .post(format!("{}/mcp/localmodel.embed", url.trim_end_matches('/')))
                    .json(&body)
                    .send()
                    .await
                {
                    if let Ok(v) = resp.json::<Value>().await {
                        if let Some(arr) = v.get("embedding").and_then(|x| x.as_array()) {
                            return arr.iter().filter_map(|x| x.as_f64().map(|f| f as f32)).collect();
                        }
                    }
                }
            }
        }
        text.chars().take(16).map(|c| (c as u32 % 97) as f32 / 97.0).collect()
    }
}

fn classify_stub(text: &str, category: &str) -> (String, f64, String, String, bool) {
    let t = text.to_lowercase();
    if category == "scheduler" {
        return ("heartbeat".into(), 0.99, "low".into(), "local".into(), false);
    }
    if category == "notification" || t.contains("notification") {
        return ("notification.triage".into(), 0.9, "low".into(), "local".into(), false);
    }
    let intent = if t.contains("calc") || t.contains("math") || t.contains("+") {
        "calculator"
    } else if t.contains("play") || t.contains("music") || t.contains("video") {
        "media_control"
    } else if t.contains("weather") || t.contains("time") || t.contains("date") {
        "query"
    } else if t.contains("build") || t.contains("create") || t.contains("make") || t.contains("show") {
        "synthesize"
    } else if t.contains("download") || t.contains("file") {
        "filesystem"
    } else {
        "generic"
    };
    (
        intent.to_string(),
        0.85,
        complexity_for(intent),
        routing_for(intent),
        requires_cloud(intent),
    )
}

fn complexity_for(intent: &str) -> String {
    match intent {
        "synthesize" | "search" => "high".into(),
        "calculator" | "filesystem" => "medium".into(),
        _ => "low".into(),
    }
}

fn routing_for(intent: &str) -> String {
    if requires_cloud(intent) {
        "cloud".to_string()
    } else {
        "local".to_string()
    }
}

fn requires_cloud(intent: &str) -> bool {
    matches!(intent, "synthesize" | "research")
}
