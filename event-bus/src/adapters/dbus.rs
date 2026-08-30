//! D-Bus adapter: monitors notifications and login events via native zbus.

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
use tracing::{info, warn};
use zbus::message::Type;
use zbus::{Connection, MatchRule, Message, MessageStream};

const NOTIFY_INTERFACE: &str = "org.freedesktop.Notifications";
const LOGIN_INTERFACE: &str = "org.freedesktop.login1.Manager";

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    tokio::spawn(monitor_notify());
    tokio::spawn(monitor_prepare_sleep());
}

async fn monitor_notify() {
    loop {
        match run_notify_stream().await {
            Ok(()) => warn!("dbus notify stream ended; restarting in 5s"),
            Err(e) => warn!("dbus notify adapter unavailable: {e}; retry in 30s"),
        }
        tokio::time::sleep(retry_delay()).await;
    }
}

async fn monitor_prepare_sleep() {
    loop {
        match run_prepare_sleep_stream().await {
            Ok(()) => warn!("dbus prepare_sleep stream ended; restarting in 5s"),
            Err(e) => warn!("dbus prepare_sleep adapter unavailable: {e}; retry in 30s"),
        }
        tokio::time::sleep(retry_delay()).await;
    }
}

async fn run_notify_stream() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let rule = notify_match_rule().map_err(|e| e.to_string())?;
    let mut stream = MessageStream::for_match_rule(rule, &conn, Some(32))
        .await
        .map_err(|e| e.to_string())?;
    info!("zbus notify monitor started");
    while let Some(msg) = stream.next().await {
        let msg = msg.map_err(|e| e.to_string())?;
        if let Some(payload) = parse_notify(&msg) {
            publish_event("notification", "desktop.notify", payload).await;
        }
    }
    Ok(())
}

async fn run_prepare_sleep_stream() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let rule = prepare_sleep_match_rule().map_err(|e| e.to_string())?;
    let mut stream = MessageStream::for_match_rule(rule, &conn, Some(8))
        .await
        .map_err(|e| e.to_string())?;
    info!("zbus prepare_sleep monitor started");
    while let Some(msg) = stream.next().await {
        let msg = msg.map_err(|e| e.to_string())?;
        if let Some(payload) = parse_prepare_sleep(&msg) {
            publish_event("system", "login.prepare_sleep", payload).await;
        }
    }
    Ok(())
}

fn notify_match_rule() -> zbus::Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(NOTIFY_INTERFACE)?
        .member("Notify")?
        .build())
}

fn prepare_sleep_match_rule() -> zbus::Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(LOGIN_INTERFACE)?
        .member("PrepareForSleep")?
        .build())
}

fn parse_notify(msg: &Message) -> Option<Value> {
    let body = msg.body();
    let (app_name, replaces_id, app_icon, summary, body_text): (
        &str,
        u32,
        &str,
        &str,
        &str,
    ) = body.deserialize().ok()?;
    Some(notify_payload(app_name, replaces_id, app_icon, summary, body_text))
}

fn parse_prepare_sleep(msg: &Message) -> Option<Value> {
    let body = msg.body();
    let sleeping: bool = body.deserialize().ok()?;
    Some(prepare_sleep_payload(sleeping))
}

fn notify_payload(
    app_name: &str,
    replaces_id: u32,
    app_icon: &str,
    summary: &str,
    body: &str,
) -> Value {
    json!({
        "app_name": app_name,
        "replaces_id": replaces_id,
        "app_icon": app_icon,
        "summary": summary,
        "body": body,
    })
}

fn prepare_sleep_payload(sleeping: bool) -> Value {
    json!({
        "sleeping": sleeping,
        "raw": format!("PrepareForSleep({sleeping})"),
    })
}

fn retry_delay() -> std::time::Duration {
    std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_match_rule_targets_freedesktop_notifications() {
        let rule = notify_match_rule().unwrap();
        let rule_str = rule.to_string();
        assert!(rule_str.contains("org.freedesktop.Notifications"));
        assert!(rule_str.contains("Notify"));
    }

    #[test]
    fn prepare_sleep_match_rule_targets_login_manager() {
        let rule = prepare_sleep_match_rule().unwrap();
        let rule_str = rule.to_string();
        assert!(rule_str.contains("org.freedesktop.login1.Manager"));
        assert!(rule_str.contains("PrepareForSleep"));
    }

    #[test]
    fn notify_payload_includes_summary_and_body() {
        let payload = notify_payload("app", 0, "icon", "Title", "Details");
        assert_eq!(payload["summary"], "Title");
        assert_eq!(payload["body"], "Details");
        assert_eq!(payload["app_name"], "app");
    }

    #[test]
    fn prepare_sleep_payload_records_sleeping_flag() {
        let payload = prepare_sleep_payload(true);
        assert_eq!(payload["sleeping"], true);
        assert!(payload["raw"].as_str().unwrap().contains("true"));
    }
}
