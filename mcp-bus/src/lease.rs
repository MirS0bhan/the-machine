//! Fast-path MCP leases for hot loops (bypass repeated bus resolution).
//!
//! G12: by default leases are metadata — we advertise the handler socket so
//! callers can skip registry lookup. When `THE_MACHINE_LEASE_FAST_PATH=1`, the
//! bus binds `{socket_dir}/leases/<id>.sock` and relays the leased method.

use dashmap::DashMap;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

pub struct LeaseRecord {
    pub lease_id: String,
    pub method: String,
    pub handler: String,
    pub handler_socket: String,
    pub manifest_ref: Option<String>,
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

    pub fn create(
        &self,
        method: &str,
        handler: &str,
        manifest_ref: Option<String>,
        ttl_secs: Option<u64>,
    ) -> Value {
        let lease_id = Uuid::new_v4().to_string();
        let ttl = Duration::from_secs(ttl_secs.unwrap_or(self.default_ttl.as_secs()));
        let handler_socket = common::component_socket(handler);
        self.leases.insert(
            lease_id.clone(),
            LeaseRecord {
                lease_id: lease_id.clone(),
                method: method.to_string(),
                handler: handler.to_string(),
                handler_socket: handler_socket.clone(),
                manifest_ref: manifest_ref.clone(),
                expires_at: Instant::now() + ttl,
            },
        );
        json!({
            "lease_id": lease_id,
            "method": method,
            "handler": handler,
            "handler_socket": handler_socket,
            "fast_path": false,
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
                    handler_socket: r.handler_socket.clone(),
                    manifest_ref: r.manifest_ref.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lease_does_not_advertise_unbound_socket() {
        let mgr = LeaseManager::new(60);
        let v = mgr.create("calc.add", "lambda-server", None, Some(45));
        assert_eq!(v["fast_path"], false);
        assert!(v.get("socket_path").is_none());
        assert_eq!(v["handler"], "lambda-server");
        let sock = v["handler_socket"].as_str().unwrap();
        assert!(sock.ends_with("/lambda-server.sock"));
        assert_eq!(v["ttl_secs"], 45);
    }
}
