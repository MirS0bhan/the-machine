//! AT-SPI-oriented D-Bus bridge (P2).
//!
//! Registers `org.themachine.A11y` on the session bus and optionally announces
//! to `org.a11y.Bus` when present. Exports the AUIL tree as AT-SPI-shaped
//! roles/names/states (same mapping as `a11y::serialize_tree`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tracing::{info, warn};
use zbus::{connection::Builder, interface, Connection};

use crate::a11y;
use crate::UiTree;

static RUNNING: AtomicBool = AtomicBool::new(false);

pub fn is_running() -> bool {
    RUNNING.load(Ordering::Relaxed)
}

#[derive(Clone)]
struct A11yRoot {
    tree: Arc<tokio::sync::Mutex<UiTree>>,
    snapshot: Arc<Mutex<Value>>,
}

#[interface(name = "org.themachine.A11y")]
impl A11yRoot {
    async fn get_tree(&self) -> String {
        if let Ok(t) = self.tree.try_lock() {
            let v = a11y::serialize_tree(&t);
            if let Ok(mut g) = self.snapshot.lock() {
                *g = v.clone();
            }
            return v.to_string();
        }
        self.snapshot
            .lock()
            .map(|g| g.to_string())
            .unwrap_or_else(|_| "{}".into())
    }

    async fn get_role(&self, id: &str) -> String {
        find_role(&self.cached_tree(), id).unwrap_or_else(|| "invalid".into())
    }

    async fn get_name(&self, id: &str) -> String {
        find_name(&self.cached_tree(), id).unwrap_or_default()
    }

    async fn get_children(&self, id: &str) -> Vec<String> {
        find_children(&self.cached_tree(), id)
    }

    async fn refresh(&self) -> bool {
        if let Ok(t) = self.tree.try_lock() {
            let v = a11y::serialize_tree(&t);
            if let Ok(mut g) = self.snapshot.lock() {
                *g = v;
            }
            return true;
        }
        false
    }
}

impl A11yRoot {
    fn cached_tree(&self) -> Value {
        self.snapshot
            .lock()
            .map(|g| g.clone())
            .unwrap_or(Value::Null)
    }
}

fn find_role(tree: &Value, id: &str) -> Option<String> {
    find_node(tree, id)?
        .get("role")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn find_name(tree: &Value, id: &str) -> Option<String> {
    find_node(tree, id)?
        .get("name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn find_children(tree: &Value, id: &str) -> Vec<String> {
    let Some(node) = find_node(tree, id) else {
        return Vec::new();
    };
    node.get("children")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn find_node<'a>(tree: &'a Value, id: &str) -> Option<&'a Value> {
    if tree.get("id").and_then(|v| v.as_str()) == Some(id) {
        return Some(tree);
    }
    let children = tree.get("children")?.as_array()?;
    for c in children {
        if let Some(found) = find_node(c, id) {
            return Some(found);
        }
    }
    None
}

/// Spawn a background task that owns the D-Bus connection. Returns false if
/// the session bus is unavailable (tests / headless CI).
pub async fn try_start(tree: Arc<tokio::sync::Mutex<UiTree>>) -> bool {
    if std::env::var("THE_MACHINE_ATSPI").ok().as_deref() == Some("0") {
        info!("AT-SPI bridge disabled via THE_MACHINE_ATSPI=0");
        return false;
    }
    let snapshot = {
        let t = tree.lock().await;
        Arc::new(Mutex::new(a11y::serialize_tree(&t)))
    };
    let root = A11yRoot {
        tree: tree.clone(),
        snapshot,
    };
    match Builder::session()
        .and_then(|b| {
            b.name("org.themachine.A11y")
                .map_err(Into::into)
        }) {
        Ok(builder) => match builder.serve_at("/org/themachine/A11y", root) {
            Ok(builder) => match builder.build().await {
                Ok(conn) => {
                    info!("AT-SPI bridge listening on org.themachine.A11y");
                    announce_a11y_bus(&conn).await;
                    RUNNING.store(true, Ordering::Relaxed);
                    // Keep connection alive for process lifetime.
                    tokio::spawn(async move {
                        loop {
                            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
                            let _ = &conn;
                        }
                    });
                    true
                }
                Err(e) => {
                    warn!("AT-SPI D-Bus build failed: {e}");
                    false
                }
            },
            Err(e) => {
                warn!("AT-SPI serve_at failed: {e}");
                false
            }
        },
        Err(e) => {
            warn!("AT-SPI session bus unavailable: {e}");
            false
        }
    }
}

async fn announce_a11y_bus(conn: &Connection) {
    // Best-effort: register with the at-spi registry if the desktop provides one.
    let _ = conn
        .call_method(
            Some("org.a11y.Bus"),
            "/org/a11y/bus",
            Some("org.a11y.Status"),
            "IsEnabled",
            &(),
        )
        .await;
}

pub fn status() -> Value {
    serde_json::json!({
        "dbus": is_running(),
        "well_known_name": "org.themachine.A11y",
        "object_path": "/org/themachine/A11y",
        "interface": "org.themachine.A11y",
        "atspi_shaped": true,
        "atspi_registry": "best-effort",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn finds_nested_role() {
        let tree = json!({
            "id": "ui.root",
            "role": "panel",
            "children": [{ "id": "ui.send", "role": "button", "name": "Send", "children": [] }]
        });
        assert_eq!(find_role(&tree, "ui.send").as_deref(), Some("button"));
        assert_eq!(find_children(&tree, "ui.root"), vec!["ui.send".to_string()]);
    }
}
