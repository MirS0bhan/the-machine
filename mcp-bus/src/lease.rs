//! Fast-path MCP leases for hot loops (bypass repeated bus resolution).

use dashmap::DashMap;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct LeaseRecord {
    pub lease_id: String,
    pub method: String,
    pub handler: String,
    pub socket_path: String,
    pub expires_at: Instant,
}

pub struct LeaseManager {
    leases: DashMap<String, LeaseRecord>,
    default_ttl: Duration,
}

impl LeaseManager {
    pub fn new(default_ttl_secs: u64) -> Self {
        LeaseManager {
            leases: DashMap::new(),
            default_ttl: Duration::from_secs(default_ttl_secs.max(30)),
        }
    }

    pub fn create(&self, method: &str, handler: &str, ttl_secs: Option<u64>) -> Value {
        let lease_id = Uuid::new_v4().to_string();
        let ttl = Duration::from_secs(ttl_secs.unwrap_or(self.default_ttl.as_secs()));
        let socket_path = format!("/run/the-machine/leases/{}.sock", lease_id);
        self.leases.insert(
            lease_id.clone(),
            LeaseRecord {
                lease_id: lease_id.clone(),
                method: method.to_string(),
                handler: handler.to_string(),
                socket_path: socket_path.clone(),
                expires_at: Instant::now() + ttl,
            },
        );
        json!({
            "lease_id": lease_id,
            "method": method,
            "handler": handler,
            "socket_path": socket_path,
            "ttl_secs": ttl.as_secs(),
        })
    }

    pub fn renew(&self, lease_id: &str, ttl_secs: Option<u64>) -> Option<Value> {
        let ttl = Duration::from_secs(ttl_secs.unwrap_or(self.default_ttl.as_secs()));
        self.leases.get_mut(lease_id).map(|mut rec| {
            rec.expires_at = Instant::now() + ttl;
            json!({ "lease_id": lease_id, "ttl_secs": ttl.as_secs(), "renewed": true })
        })
    }

    pub fn get(&self, lease_id: &str) -> Option<LeaseRecord> {
        self.leases.get(lease_id).and_then(|r| {
            if r.expires_at < Instant::now() {
                drop(r);
                self.leases.remove(lease_id);
                None
            } else {
                Some(LeaseRecord {
                    lease_id: r.lease_id.clone(),
                    method: r.method.clone(),
                    handler: r.handler.clone(),
                    socket_path: r.socket_path.clone(),
                    expires_at: r.expires_at,
                })
            }
        })
    }

    pub fn purge_expired(&self) {
        self.leases.retain(|_, v| v.expires_at >= Instant::now());
    }
}

pub type SharedLeases = Arc<LeaseManager>;
