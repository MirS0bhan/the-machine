//! D-Bus adapter: monitors notifications and login events via native zbus.

use super::publish_event;
use futures::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use tracing::{info, warn};
use zbus::message::Type;
use zbus::{Connection, MatchRule, Message, MessageStream};

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
        let delay = if cfg!(test) { 1 } else { 30 };
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
    }
}

async fn listen() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    info!("dbus adapter connected to system bus");

    let notify_rule = notify_match_rule()?;
    let sleep_rule = sleep_match_rule()?;

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(16))
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(16))
        .await
        .map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_notify(msg).await,
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_prepare_sleep(msg).await,
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
        }
    }
}

fn notify_match_rule() -> Result<MatchRule<'static>, String> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.Notifications")
        .map_err(|e| e.to_string())?
        .member("Notify")
        .map_err(|e| e.to_string())?
        .build())
}

fn sleep_match_rule() -> Result<MatchRule<'static>, String> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.login1.Manager")
        .map_err(|e| e.to_string())?
        .member("PrepareForSleep")
        .map_err(|e| e.to_string())?
        .build())
}

type NotifyArgs<'a> = (
    String,
    u32,
    String,
    String,
    String,
    Vec<String>,
    HashMap<String, zbus::zvariant::Value<'a>>,
    i32,
);

fn notify_summary(msg: &Message) -> String {
    msg.body()
        .deserialize::<NotifyArgs<'_>>()
        .ok()
        .map(|(_, _, _, summary, _, _, _, _)| summary)
        .unwrap_or_default()
}

fn prepare_sleep_flag(msg: &Message) -> Option<bool> {
    msg.body().deserialize().ok()
}

async fn handle_notify(msg: Message) {
    let summary = notify_summary(&msg);
    publish_event(
        "notification",
        "desktop.notify",
        json!({
            "raw": format!("{msg:?}"),
            "summary": summary,
        }),
    )
    .await;
}

async fn handle_prepare_sleep(msg: Message) {
    let sleep = prepare_sleep_flag(&msg);
    publish_event(
        "system",
        "login.prepare_sleep",
        json!({
            "raw": format!("{msg:?}"),
            "sleep": sleep,
        }),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_match_rule_matches_dbus_monitor_filter() {
        let rule = notify_match_rule().unwrap();
        assert!(rule.to_string().contains("org.freedesktop.Notifications"));
        assert!(rule.to_string().contains("Notify"));
    }

    #[test]
    fn sleep_match_rule_matches_dbus_monitor_filter() {
        let rule = sleep_match_rule().unwrap();
        assert!(rule.to_string().contains("org.freedesktop.login1.Manager"));
        assert!(rule.to_string().contains("PrepareForSleep"));
    }
}
