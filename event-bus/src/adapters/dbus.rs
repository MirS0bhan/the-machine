//! D-Bus adapter: monitors desktop notifications and login sleep events via zbus.

use super::publish_event;
use futures::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use tracing::{info, warn};
use zbus::match_rule::MatchRule;
use zbus::message::Type;
use zbus::{Connection, Message, MessageStream};
use zbus::zvariant::OwnedValue;

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        let delay_secs = match monitor_signals().await {
            Ok(()) => {
                warn!("zbus signal monitor exited; restarting in 5s");
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
    let conn = Connection::system().await.map_err(|e| e.to_string())?;

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

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, None)
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, None)
        .await
        .map_err(|e| e.to_string())?;

    info!("zbus D-Bus signal monitor started");

    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_notify(&msg).await,
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_prepare_sleep(&msg).await,
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
        }
    }
}

async fn handle_notify(msg: &Message) {
    let payload = notify_payload(msg).unwrap_or_else(|_| json!({}));
    publish_event("notification", "desktop.notify", payload).await;
}

async fn handle_prepare_sleep(msg: &Message) {
    let payload = prepare_sleep_payload(msg).unwrap_or_else(|_| json!({}));
    publish_event("system", "login.prepare_sleep", payload).await;
}

fn notify_payload(msg: &Message) -> Result<serde_json::Value, zbus::Error> {
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
    let (app_name, _, app_icon, summary, body, ..): NotifyBody = msg.body().deserialize()?;
    Ok(json!({
        "app_name": app_name,
        "app_icon": app_icon,
        "summary": summary,
        "body": body,
    }))
}

fn prepare_sleep_payload(msg: &Message) -> Result<serde_json::Value, zbus::Error> {
    let (start,): (bool,) = msg.body().deserialize()?;
    Ok(json!({ "start": start }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_match_rule_matches_dbus_monitor_filter() {
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.Notifications")
            .unwrap()
            .member("Notify")
            .unwrap()
            .build();
        assert_eq!(
            rule.to_string(),
            "type='signal',interface='org.freedesktop.Notifications',member='Notify'"
        );
    }

    #[test]
    fn prepare_sleep_match_rule_matches_dbus_monitor_filter() {
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.login1.Manager")
            .unwrap()
            .member("PrepareForSleep")
            .unwrap()
            .build();
        assert_eq!(
            rule.to_string(),
            "type='signal',interface='org.freedesktop.login1.Manager',member='PrepareForSleep'"
        );
    }
}
