//! D-Bus adapter: native zbus subscriptions for desktop notifications and login events.

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{info, warn};
use zbus::message::{Message, Type};
use zbus::match_rule::MatchRule;
use zbus::{Connection, MessageStream};

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match monitor_signals().await {
            Ok(()) => warn!("dbus signal stream ended; restarting in 30s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
    }
}

async fn monitor_signals() -> Result<(), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("system bus connect failed: {e}"))?;

    let notify_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.Notifications")
        .map_err(|e| e.to_string())?
        .member("Notify")
        .map_err(|e| e.to_string())?
        .build();

    let sleep_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.login1.Manager")
        .map_err(|e| e.to_string())?
        .member("PrepareForSleep")
        .map_err(|e| e.to_string())?
        .build();

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(8))
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(8))
        .await
        .map_err(|e| e.to_string())?;

    info!("dbus signal subscriptions active (zbus)");

    loop {
        tokio::select! {
            msg = notify_stream.next() => match msg {
                Some(Ok(msg)) => {
                    if let Some(payload) = parse_notify(&msg) {
                        publish_event("notification", "desktop.notify", payload).await;
                    }
                }
                Some(Err(e)) => return Err(format!("notify stream error: {e}")),
                None => return Ok(()),
            },
            msg = sleep_stream.next() => match msg {
                Some(Ok(msg)) => {
                    if let Some(payload) = parse_prepare_for_sleep(&msg) {
                        publish_event("system", "login.prepare_sleep", payload).await;
                    }
                }
                Some(Err(e)) => return Err(format!("sleep stream error: {e}")),
                None => return Ok(()),
            },
        }
    }
}

/// Parse `org.freedesktop.Notifications.Notify` body into an event payload.
fn parse_notify(msg: &Message) -> Option<Value> {
    let body = msg.body();
    let (
        app_name,
        replaces_id,
        app_icon,
        summary,
        body_text,
        actions,
        hints,
        expire_timeout,
    ): (
        &str,
        u32,
        &str,
        &str,
        &str,
        Vec<&str>,
        HashMap<&str, zbus::zvariant::Value<'_>>,
        i32,
    ) = body.deserialize().ok()?;

    Some(json!({
        "app_name": app_name,
        "replaces_id": replaces_id,
        "app_icon": app_icon,
        "summary": summary,
        "body": body_text,
        "actions": actions,
        "hints": hints.keys().collect::<Vec<_>>(),
        "expire_timeout": expire_timeout,
    }))
}

/// Parse `org.freedesktop.login1.Manager.PrepareForSleep` body into an event payload.
fn parse_prepare_for_sleep(msg: &Message) -> Option<Value> {
    let active: bool = msg.body().deserialize().ok()?;
    Some(json!({ "active": active }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::Value;

    fn notify_message(
        app_name: &str,
        summary: &str,
        body: &str,
    ) -> Message {
        Message::signal(
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
        )
        .unwrap()
        .build(&(app_name, 0u32, "", summary, body, Vec::<&str>::new(), HashMap::<&str, Value>::new(), 0i32))
        .unwrap()
    }

    fn prepare_for_sleep_message(active: bool) -> Message {
        Message::signal(
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
            "PrepareForSleep",
        )
        .unwrap()
        .build(&(active,))
        .unwrap()
    }

    #[test]
    fn parse_notify_extracts_summary_and_body() {
        let msg = notify_message("app", "Hello", "World");
        let payload = parse_notify(&msg).expect("notify payload");
        assert_eq!(payload["app_name"], "app");
        assert_eq!(payload["summary"], "Hello");
        assert_eq!(payload["body"], "World");
    }

    #[test]
    fn parse_prepare_for_sleep_extracts_active_flag() {
        let msg = prepare_for_sleep_message(true);
        let payload = parse_prepare_for_sleep(&msg).expect("sleep payload");
        assert_eq!(payload["active"], true);

        let msg = prepare_for_sleep_message(false);
        let payload = parse_prepare_for_sleep(&msg).expect("sleep payload");
        assert_eq!(payload["active"], false);
    }
}
