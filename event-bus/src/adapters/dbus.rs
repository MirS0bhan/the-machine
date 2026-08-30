//! D-Bus adapter: native zbus subscription for notifications and login events.

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
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
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
    }
}

async fn listen() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    info!("dbus system bus connected (zbus)");

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
                    Some(Ok(msg)) => handle_prepare_for_sleep(msg).await,
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
        }
    }
}

async fn handle_notify(msg: Message) {
    let payload = notify_payload(&msg).unwrap_or_else(|e| {
        warn!("failed to decode Notify signal: {}", e);
        json!({ "decode_error": e.to_string() })
    });
    publish_event("notification", "desktop.notify", payload).await;
}

async fn handle_prepare_for_sleep(msg: Message) {
    let payload = prepare_for_sleep_payload(&msg).unwrap_or_else(|e| {
        warn!("failed to decode PrepareForSleep signal: {}", e);
        json!({ "decode_error": e.to_string() })
    });
    publish_event("system", "login.prepare_sleep", payload).await;
}

/// Decode `org.freedesktop.Notifications.Notify` body fields we care about.
fn notify_payload(msg: &Message) -> Result<Value, String> {
    let body = msg.body();
    let (
        app_name,
        _replaces_id,
        _app_icon,
        summary,
        body_text,
        _actions,
        _hints,
        _expire_timeout,
    ): (
        &str,
        u32,
        &str,
        &str,
        &str,
        Vec<&str>,
        std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        i32,
    ) = body.deserialize().map_err(|e| e.to_string())?;
    Ok(json!({
        "app_name": app_name,
        "summary": summary,
        "body": body_text,
    }))
}

/// Decode `org.freedesktop.login1.Manager.PrepareForSleep` boolean argument.
fn prepare_for_sleep_payload(msg: &Message) -> Result<Value, String> {
    let sleep: bool = msg.body().deserialize().map_err(|e| e.to_string())?;
    Ok(json!({ "sleep": sleep }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::message::Type;

    #[test]
    fn match_rules_build_for_expected_interfaces() {
        let notify = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.Notifications")
            .unwrap()
            .member("Notify")
            .unwrap()
            .build();
        assert!(format!("{notify}").contains("org.freedesktop.Notifications"));
        let sleep = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.login1.Manager")
            .unwrap()
            .member("PrepareForSleep")
            .unwrap()
            .build();
        assert!(format!("{sleep}").contains("PrepareForSleep"));
    }
}
