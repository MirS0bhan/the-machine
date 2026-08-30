//! D-Bus adapter: monitors desktop notifications and login sleep events via zbus.

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::{info, warn};
use zbus::message::Type;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, Message, MessageStream};

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match monitor_signals().await {
            Ok(()) => warn!("dbus connection closed; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) {
            1
        } else {
            30
        }))
        .await;
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

    info!("dbus signal monitor started (zbus system bus)");
    let notify_conn = conn.clone();
    let sleep_conn = conn;
    tokio::select! {
        result = run_match(notify_conn, notify_rule) => result,
        result = run_match(sleep_conn, sleep_rule) => result,
    }
}

async fn run_match(conn: Connection, rule: MatchRule<'static>) -> Result<(), String> {
    let mut stream = MessageStream::for_match_rule(rule, &conn, None)
        .await
        .map_err(|e| format!("dbus match subscribe failed: {e}"))?;
    while let Some(msg) = stream.next().await {
        let msg = msg.map_err(|e| format!("dbus message read failed: {e}"))?;
        handle_message(&msg).await;
    }
    Ok(())
}

async fn handle_message(msg: &Message) {
    let header = msg.header();
    let interface = header.interface().map(|i| i.as_str());
    let member = header.member().map(|m| m.as_str());

    match (interface, member) {
        (Some("org.freedesktop.Notifications"), Some("Notify")) => {
            publish_event(
                "notification",
                "desktop.notify",
                notify_payload(msg).unwrap_or_else(|e| {
                    json!({ "parse_error": e, "interface": interface, "member": member })
                }),
            )
            .await;
        }
        (Some("org.freedesktop.login1.Manager"), Some("PrepareForSleep")) => {
            publish_event(
                "system",
                "login.prepare_sleep",
                sleep_payload(msg).unwrap_or_else(|e| {
                    json!({ "parse_error": e, "interface": interface, "member": member })
                }),
            )
            .await;
        }
        _ => {}
    }
}

type NotifyBody = (
    String,
    u32,
    String,
    String,
    String,
    Vec<String>,
    HashMap<String, OwnedValue>,
    i32,
);

fn notify_payload(msg: &Message) -> Result<Value, String> {
    let (app_name, replaces_id, app_icon, summary, body, ..): NotifyBody = msg
        .body()
        .deserialize()
        .map_err(|e| format!("Notify body decode failed: {e}"))?;
    Ok(json!({
        "app_name": app_name,
        "replaces_id": replaces_id,
        "app_icon": app_icon,
        "summary": summary,
        "body": body,
    }))
}

fn sleep_payload(msg: &Message) -> Result<Value, String> {
    let active: bool = msg
        .body()
        .deserialize()
        .map_err(|e| format!("PrepareForSleep body decode failed: {e}"))?;
    Ok(json!({ "active": active }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_payload_from_message_body() {
        let msg = Message::signal(
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
        )
        .unwrap()
        .build(&(
            "app",
            0u32,
            "icon",
            "Summary",
            "Body",
            Vec::<String>::new(),
            HashMap::<String, OwnedValue>::new(),
            0i32,
        ))
        .unwrap();
        let payload = notify_payload(&msg).unwrap();
        assert_eq!(payload["summary"], "Summary");
        assert_eq!(payload["body"], "Body");
        assert_eq!(payload["app_name"], "app");
    }

    #[test]
    fn sleep_payload_from_message_body() {
        let msg = Message::signal(
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
            "PrepareForSleep",
        )
        .unwrap()
        .build(&true)
        .unwrap();
        let payload = sleep_payload(&msg).unwrap();
        assert_eq!(payload["active"], true);
    }
}
