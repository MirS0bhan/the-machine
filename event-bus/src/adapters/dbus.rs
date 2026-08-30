//! D-Bus adapter: monitors desktop notifications and login sleep events via native zbus.

use super::publish_event;
use futures::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use tracing::{info, warn};
use zbus::message::Type;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, Message, MessageStream};

type NotifyArgs = (
    String,
    u32,
    String,
    String,
    String,
    Vec<String>,
    HashMap<String, OwnedValue>,
    i32,
);

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match run_session().await {
            Ok(()) => warn!("dbus session ended; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        let delay = if cfg!(test) { 1 } else { 30 };
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
    }
}

async fn run_session() -> Result<(), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("system bus connect: {e}"))?;

    let mut notify_stream = signal_stream(&conn, notify_match_rule()).await?;
    let mut sleep_stream = signal_stream(&conn, prepare_sleep_match_rule()).await?;

    info!("dbus adapter connected (native zbus)");

    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        publish_event("notification", "desktop.notify", notify_payload(&msg)).await;
                    }
                    Some(Err(e)) => return Err(format!("notify stream: {e}")),
                    None => return Ok(()),
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        publish_event(
                            "system",
                            "login.prepare_sleep",
                            prepare_sleep_payload(&msg),
                        )
                        .await;
                    }
                    Some(Err(e)) => return Err(format!("sleep stream: {e}")),
                    None => return Ok(()),
                }
            }
        }
    }
}

async fn signal_stream(
    conn: &Connection,
    rule: MatchRule<'static>,
) -> Result<MessageStream, String> {
    MessageStream::for_match_rule(rule, conn, None)
        .await
        .map_err(|e| e.to_string())
}

fn notify_match_rule() -> MatchRule<'static> {
    MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.Notifications")
        .expect("valid interface")
        .member("Notify")
        .expect("valid member")
        .build()
}

fn prepare_sleep_match_rule() -> MatchRule<'static> {
    MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.login1.Manager")
        .expect("valid interface")
        .member("PrepareForSleep")
        .expect("valid member")
        .build()
}

fn notify_payload(msg: &Message) -> serde_json::Value {
    match msg.body().deserialize::<NotifyArgs>() {
        Ok((app_name, replaces_id, icon, summary, body, ..)) => json!({
            "app_name": app_name,
            "replaces_id": replaces_id,
            "icon": icon,
            "summary": summary,
            "body": body,
        }),
        Err(e) => json!({
            "member": "Notify",
            "summary": extract_field_fallback(msg),
            "parse_error": e.to_string(),
        }),
    }
}

fn prepare_sleep_payload(msg: &Message) -> serde_json::Value {
    match msg.body().deserialize::<(bool,)>() {
        Ok((sleeping,)) => json!({ "sleeping": sleeping }),
        Err(e) => json!({
            "member": "PrepareForSleep",
            "parse_error": e.to_string(),
        }),
    }
}

fn extract_field_fallback(msg: &Message) -> String {
    truncate_chars(&msg.to_string(), 120)
}

fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_match_rule_targets_freedesktop_notifications() {
        let rule = notify_match_rule().to_string();
        assert!(rule.contains("type='signal'"));
        assert!(rule.contains("interface='org.freedesktop.Notifications'"));
        assert!(rule.contains("member='Notify'"));
    }

    #[test]
    fn prepare_sleep_match_rule_targets_login_manager() {
        let rule = prepare_sleep_match_rule().to_string();
        assert!(rule.contains("type='signal'"));
        assert!(rule.contains("interface='org.freedesktop.login1.Manager'"));
        assert!(rule.contains("member='PrepareForSleep'"));
    }

    #[test]
    fn truncate_chars_limits_length() {
        let long = "x".repeat(200);
        assert_eq!(truncate_chars(&long, 120).len(), 120);
    }
}
