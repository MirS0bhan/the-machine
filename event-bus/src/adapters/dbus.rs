//! D-Bus adapter: native zbus signal monitoring (no external `dbus-monitor` binary).

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
use tracing::{info, warn};
use zbus::Connection;

#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[zbus(signal)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        expire_timeout: i32,
    ) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, active: bool) -> zbus::Result<()>;
}

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        let delay_secs = match monitor_signals().await {
            Ok(()) => {
                warn!("dbus signal streams ended; reconnecting in 5s");
                if cfg!(test) { 1 } else { 5 }
            }
            Err(e) => {
                warn!("dbus adapter unavailable: {}; retry in 30s", e);
                if cfg!(test) { 1 } else { 30 }
            }
        };
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
    }
}

async fn monitor_signals() -> Result<(), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("system bus connect: {e}"))?;

    let notifications = NotificationsProxy::new(&conn)
        .await
        .map_err(|e| format!("notifications proxy: {e}"))?;
    let login = Login1ManagerProxy::new(&conn)
        .await
        .map_err(|e| format!("login1 proxy: {e}"))?;

    let mut notify_stream = notifications
        .receive_notify()
        .await
        .map_err(|e| format!("notify stream: {e}"))?;
    let mut sleep_stream = login
        .receive_prepare_for_sleep()
        .await
        .map_err(|e| format!("prepare_for_sleep stream: {e}"))?;

    info!("dbus adapter listening (zbus)");

    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(msg) => {
                        let args = msg.args().map_err(|e| format!("notify args: {e}"))?;
                        publish_event(
                            "notification",
                            "desktop.notify",
                            notify_payload(
                                args.app_name(),
                                *args.replaces_id(),
                                args.summary(),
                                args.body(),
                            ),
                        )
                        .await;
                    }
                    None => return Ok(()),
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(msg) => {
                        let args = msg.args().map_err(|e| format!("prepare_for_sleep args: {e}"))?;
                        publish_event(
                            "system",
                            "login.prepare_sleep",
                            prepare_sleep_payload(*args.active()),
                        )
                        .await;
                    }
                    None => return Ok(()),
                }
            }
        }
    }
}

fn notify_payload(app_name: &str, replaces_id: u32, summary: &str, body: &str) -> Value {
    json!({
        "app_name": app_name,
        "replaces_id": replaces_id,
        "summary": summary,
        "body": body,
    })
}

fn prepare_sleep_payload(active: bool) -> Value {
    json!({ "active": active })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_payload_includes_summary_and_body() {
        let payload = notify_payload("app", 7, "Title", "Details");
        assert_eq!(payload["app_name"], "app");
        assert_eq!(payload["replaces_id"], 7);
        assert_eq!(payload["summary"], "Title");
        assert_eq!(payload["body"], "Details");
    }

    #[test]
    fn prepare_sleep_payload_reflects_active_flag() {
        assert_eq!(prepare_sleep_payload(true)["active"], true);
        assert_eq!(prepare_sleep_payload(false)["active"], false);
    }
}
