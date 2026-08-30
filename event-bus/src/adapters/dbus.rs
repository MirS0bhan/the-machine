//! D-Bus adapter: native zbus subscriptions for desktop notifications and login events.

use super::publish_event;
use futures_util::StreamExt;
use serde_json::json;
use zbus::zvariant::Value as DbusValue;
use std::collections::HashMap;
use tracing::{info, warn};
use zbus::message::Type;
use zbus::message::Message;
use zbus::match_rule::MatchRule;
use zbus::MessageStream;
use zbus::Connection;

const NOTIFY_INTERFACE: &str = "org.freedesktop.Notifications";
const NOTIFY_MEMBER: &str = "Notify";
const LOGIN_INTERFACE: &str = "org.freedesktop.login1.Manager";
const LOGIN_MEMBER: &str = "PrepareForSleep";

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        let delay_secs = match monitor_signals().await {
            Ok(()) => {
                warn!("dbus adapter stream ended; restarting in 5s");
                5
            }
            Err(e) => {
                warn!("dbus adapter unavailable: {}; retry in 30s", e);
                30
            }
        };
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) {
            1
        } else {
            delay_secs
        }))
        .await;
    }
}

async fn monitor_signals() -> Result<(), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("system bus connect failed: {e}"))?;
    info!("dbus adapter connected (zbus system bus)");

    let notify_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(NOTIFY_INTERFACE)
        .map_err(|e| e.to_string())?
        .member(NOTIFY_MEMBER)
        .map_err(|e| e.to_string())?
        .build();

    let sleep_rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(LOGIN_INTERFACE)
        .map_err(|e| e.to_string())?
        .member(LOGIN_MEMBER)
        .map_err(|e| e.to_string())?
        .build();

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(16))
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(8))
        .await
        .map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            msg = notify_stream.next() => match msg {
                Some(Ok(msg)) => handle_message(&msg).await,
                Some(Err(e)) => return Err(format!("notify stream error: {e}")),
                None => return Ok(()),
            },
            msg = sleep_stream.next() => match msg {
                Some(Ok(msg)) => handle_message(&msg).await,
                Some(Err(e)) => return Err(format!("login stream error: {e}")),
                None => return Ok(()),
            },
        }
    }
}

async fn handle_message(msg: &Message) {
    let member = msg
        .header()
        .member()
        .map(|m| m.to_string())
        .unwrap_or_default();
    match member.as_str() {
        NOTIFY_MEMBER => {
            let payload = parse_notify(msg).unwrap_or_else(|e| {
                warn!("failed to parse Notify signal: {e}");
                json!({ "parse_error": e.to_string() })
            });
            publish_event("notification", "desktop.notify", payload).await;
        }
        LOGIN_MEMBER => {
            let payload = parse_prepare_for_sleep(msg).unwrap_or_else(|e| {
                warn!("failed to parse PrepareForSleep signal: {e}");
                json!({ "parse_error": e.to_string() })
            });
            publish_event("system", "login.prepare_sleep", payload).await;
        }
        _ => {}
    }
}

fn parse_notify(msg: &Message) -> zbus::Result<serde_json::Value> {
    let (
        app_name,
        replaces_id,
        app_icon,
        summary,
        body,
        _actions,
        _hints,
        expire_timeout,
    ): (
        String,
        u32,
        String,
        String,
        String,
        Vec<String>,
        HashMap<String, DbusValue<'_>>,
        i32,
    ) = msg.body().deserialize()?;

    Ok(json!({
        "app_name": app_name,
        "replaces_id": replaces_id,
        "app_icon": app_icon,
        "summary": summary,
        "body": body,
        "expire_timeout": expire_timeout,
    }))
}

fn parse_prepare_for_sleep(msg: &Message) -> zbus::Result<serde_json::Value> {
    let start: bool = msg.body().deserialize()?;
    Ok(json!({ "start": start }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_match_rule_targets_freedesktop_notifications() {
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface(NOTIFY_INTERFACE)
            .unwrap()
            .member(NOTIFY_MEMBER)
            .unwrap()
            .build();
        let rule_str = rule.to_string();
        assert!(rule_str.contains("org.freedesktop.Notifications"));
        assert!(rule_str.contains("Notify"));
    }

    #[test]
    fn login_match_rule_targets_prepare_for_sleep() {
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface(LOGIN_INTERFACE)
            .unwrap()
            .member(LOGIN_MEMBER)
            .unwrap()
            .build();
        let rule_str = rule.to_string();
        assert!(rule_str.contains("org.freedesktop.login1.Manager"));
        assert!(rule_str.contains("PrepareForSleep"));
    }
}
