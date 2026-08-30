//! D-Bus adapter: native zbus subscriptions for notifications and login events.

use super::publish_event;
use futures::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use tracing::{info, warn};
use zbus::{proxy, zvariant::Value, Connection};

#[proxy(
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
        hints: HashMap<String, Value<'_>>,
        expire_timeout: i32,
    ) -> zbus::Result<()>;
}

#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match subscribe_signals().await {
            Ok(()) => {
                warn!("dbus signal stream ended; restarting in 5s");
                tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) {
                    1
                } else {
                    5
                }))
                .await;
            }
            Err(e) => {
                warn!("dbus adapter unavailable: {}; retry in 30s", e);
                tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) {
                    1
                } else {
                    30
                }))
                .await;
            }
        }
    }
}

async fn subscribe_signals() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    info!("zbus system bus connected");

    let notifications = NotificationsProxy::new(&conn)
        .await
        .map_err(|e| e.to_string())?;
    let login = Login1ManagerProxy::new(&conn)
        .await
        .map_err(|e| e.to_string())?;

    let mut notify_stream = notifications
        .receive_notify()
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = login
        .receive_prepare_for_sleep()
        .await
        .map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(msg) => {
                        if let Ok(args) = msg.args() {
                            publish_event(
                                "notification",
                                "desktop.notify",
                                json!({
                                    "app_name": args.app_name(),
                                    "summary": args.summary(),
                                    "body": args.body(),
                                }),
                            )
                            .await;
                        }
                    }
                    None => return Ok(()),
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(msg) => {
                        if let Ok(args) = msg.args() {
                            publish_event(
                                "system",
                                "login.prepare_sleep",
                                json!({ "start": args.start() }),
                            )
                            .await;
                        }
                    }
                    None => return Ok(()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn disabled_via_env_returns_immediately() {
        std::env::set_var("THE_MACHINE_DISABLE_DBUS", "1");
        tokio::time::timeout(std::time::Duration::from_millis(100), run())
            .await
            .expect("run should return immediately when dbus is disabled");
        std::env::remove_var("THE_MACHINE_DISABLE_DBUS");
    }
}
