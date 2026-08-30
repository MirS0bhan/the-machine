//! D-Bus adapter: monitors desktop notifications and login sleep events via zbus.

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
use tracing::{info, warn};
use zbus::message::Type;
use zbus::{Connection, MatchRule, Message, MessageStream, OwnedMatchRule};

const NOTIFY_INTERFACE: &str = "org.freedesktop.Notifications";
const NOTIFY_MEMBER: &str = "Notify";
const SLEEP_INTERFACE: &str = "org.freedesktop.login1.Manager";
const SLEEP_MEMBER: &str = "PrepareForSleep";

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match monitor_signals().await {
            Ok(()) => warn!("dbus signal monitor exited; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
    }
}

async fn monitor_signals() -> Result<(), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("system bus connect failed: {e}"))?;

    let notify_rule = signal_rule(NOTIFY_INTERFACE, NOTIFY_MEMBER)?;
    let sleep_rule = signal_rule(SLEEP_INTERFACE, SLEEP_MEMBER)?;

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(8))
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(4))
        .await
        .map_err(|e| e.to_string())?;

    info!("dbus signal monitor started (zbus)");

    loop {
        tokio::select! {
            msg = notify_stream.next() => match msg {
                Some(Ok(msg)) => {
                    publish_event("notification", "desktop.notify", notify_payload(&msg)).await;
                }
                Some(Err(e)) => return Err(format!("notify stream error: {e}")),
                None => return Ok(()),
            },
            msg = sleep_stream.next() => match msg {
                Some(Ok(msg)) => {
                    publish_event("system", "login.prepare_sleep", sleep_payload(&msg)).await;
                }
                Some(Err(e)) => return Err(format!("sleep stream error: {e}")),
                None => return Ok(()),
            },
        }
    }
}

fn signal_rule(interface: &str, member: &str) -> Result<OwnedMatchRule, String> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(interface.to_string())
        .map_err(|e| e.to_string())?
        .member(member.to_string())
        .map_err(|e| e.to_string())?
        .build()
        .into())
}

fn notify_payload(msg: &Message) -> Value {
    let body = msg.body();
    match body.deserialize::<(String, u32, String, String, String)>() {
        Ok((app_name, replaces_id, app_icon, summary, body_text)) => json!({
            "app_name": app_name,
            "replaces_id": replaces_id,
            "app_icon": app_icon,
            "summary": summary,
            "body": body_text,
        }),
        Err(_) => json!({
            "summary": extract_field(&msg.to_string(), "string"),
            "raw": msg.to_string(),
        }),
    }
}

fn sleep_payload(msg: &Message) -> Value {
    let body = msg.body();
    match body.deserialize::<(bool,)>() {
        Ok((active,)) => json!({ "active": active }),
        Err(_) => json!({ "raw": msg.to_string() }),
    }
}

fn extract_field(line: &str, kind: &str) -> String {
    if line.contains(kind) {
        line.chars().take(120).collect()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_rules_match_dbus_monitor_filters() {
        let notify = signal_rule(NOTIFY_INTERFACE, NOTIFY_MEMBER).unwrap();
        assert!(notify.to_string().contains("org.freedesktop.Notifications"));
        assert!(notify.to_string().contains("Notify"));

        let sleep = signal_rule(SLEEP_INTERFACE, SLEEP_MEMBER).unwrap();
        assert!(sleep.to_string().contains("org.freedesktop.login1.Manager"));
        assert!(sleep.to_string().contains("PrepareForSleep"));
    }

    #[test]
    fn extract_field_truncates_long_lines() {
        let line = format!("string \"{}\"", "x".repeat(200));
        assert_eq!(extract_field(&line, "string").len(), 120);
    }

    #[test]
    fn extract_field_empty_when_kind_missing() {
        assert!(extract_field("no match here", "string").is_empty());
    }
}
