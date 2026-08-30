//! D-Bus adapter: native zbus subscriptions for desktop notifications and login events.

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{info, warn};
use zbus::proxy;
use zbus::zvariant::OwnedValue;

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
        hints: HashMap<String, OwnedValue>,
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
        match listen().await {
            Ok(()) => warn!("dbus connection closed; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
    }
}

async fn listen() -> Result<(), String> {
    let conn = zbus::Connection::system()
        .await
        .map_err(|e| e.to_string())?;

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

    info!("dbus adapter connected (native zbus)");

    loop {
        tokio::select! {
            signal = notify_stream.next() => {
                match signal {
                    Some(signal) => match signal.args() {
                        Ok(args) => {
                            publish_event(
                                "notification",
                                "desktop.notify",
                                notify_payload(
                                    args.app_name(),
                                    *args.replaces_id(),
                                    args.app_icon(),
                                    args.summary(),
                                    args.body(),
                                    args.actions(),
                                ),
                            )
                            .await;
                        }
                        Err(e) => warn!("notify signal parse error: {}", e),
                    },
                    None => break,
                }
            }
            signal = sleep_stream.next() => {
                match signal {
                    Some(signal) => match signal.args() {
                        Ok(args) => {
                            publish_event(
                                "system",
                                "login.prepare_sleep",
                                json!({ "start": args.start() }),
                            )
                            .await;
                        }
                        Err(e) => warn!("prepare_for_sleep parse error: {}", e),
                    },
                    None => break,
                }
            }
        }
    }
    Ok(())
}

fn notify_payload(
    app_name: &str,
    replaces_id: u32,
    app_icon: &str,
    summary: &str,
    body: &str,
    actions: &[String],
) -> Value {
    json!({
        "app_name": app_name,
        "replaces_id": replaces_id,
        "app_icon": app_icon,
        "summary": summary,
        "body": body,
        "actions": actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_payload_includes_summary_and_body() {
        let payload = notify_payload(
            "the-machine",
            0,
            "icon",
            "Hello",
            "World",
            &["default".into(), "Open".into()],
        );
        assert_eq!(payload["summary"], "Hello");
        assert_eq!(payload["body"], "World");
        assert_eq!(payload["app_name"], "the-machine");
        assert_eq!(payload["actions"][0], "default");
    }
}
