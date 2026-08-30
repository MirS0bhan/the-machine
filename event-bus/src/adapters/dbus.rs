//! D-Bus adapter: subscribes to notifications and login events via native zbus.

use super::publish_event;
use futures::StreamExt;
use serde_json::json;
use tracing::{info, warn};
use zbus::message::Type;
use zbus::zvariant::Value;
use zbus::{Connection, MatchRule, Message, MessageStream};

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match listen().await {
            Ok(()) => warn!("dbus adapter stream ended; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
    }
}

async fn listen() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    info!("dbus adapter connected to system bus");

    let mut notify = MessageStream::for_match_rule(notify_rule(), &conn, Some(16))
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep = MessageStream::for_match_rule(prepare_sleep_rule(), &conn, Some(8))
        .await
        .map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            msg = notify.next() => {
                match msg {
                    Some(Ok(msg)) => handle_notify(&msg).await,
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
            msg = sleep.next() => {
                match msg {
                    Some(Ok(msg)) => handle_prepare_sleep(&msg).await,
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
        }
    }
}

fn notify_rule() -> MatchRule<'static> {
    MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.Notifications")
        .expect("valid interface")
        .member("Notify")
        .expect("valid member")
        .build()
}

fn prepare_sleep_rule() -> MatchRule<'static> {
    MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.login1.Manager")
        .expect("valid interface")
        .member("PrepareForSleep")
        .expect("valid member")
        .build()
}

async fn handle_notify(msg: &Message) {
    match notify_payload(msg) {
        Ok(payload) => {
            publish_event("notification", "desktop.notify", payload).await;
        }
        Err(e) => warn!("failed to decode Notify signal: {}", e),
    }
}

async fn handle_prepare_sleep(msg: &Message) {
    match msg.body().deserialize::<bool>() {
        Ok(starting) => {
            publish_event(
                "system",
                "login.prepare_sleep",
                json!({ "starting": starting }),
            )
            .await;
        }
        Err(e) => warn!("failed to decode PrepareForSleep signal: {}", e),
    }
}

/// Decode `org.freedesktop.Notifications.Notify` body into a JSON payload.
fn notify_payload(msg: &Message) -> Result<serde_json::Value, String> {
    let body = msg.body();
    let structure: zbus::zvariant::Structure<'_> =
        body.deserialize().map_err(|e| e.to_string())?;
    let fields = structure.fields();
    if fields.len() < 5 {
        return Err(format!(
            "Notify signal has {} fields, expected at least 5",
            fields.len()
        ));
    }

    let app_name = field_as_str(&fields[0], "app_name")?;
    let summary = field_as_str(&fields[3], "summary")?;
    let body_text = field_as_str(&fields[4], "body")?;

    Ok(json!({
        "app_name": app_name,
        "summary": summary,
        "body": body_text,
    }))
}

fn field_as_str(value: &Value<'_>, name: &str) -> Result<String, String> {
    value
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .map_err(|_| format!("Notify field {name} is not a string"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_rule_matches_notifications_interface() {
        let rule = notify_rule();
        let s = rule.to_string();
        assert!(s.contains("org.freedesktop.Notifications"));
        assert!(s.contains("Notify"));
    }

    #[test]
    fn prepare_sleep_rule_matches_login_interface() {
        let rule = prepare_sleep_rule();
        let s = rule.to_string();
        assert!(s.contains("org.freedesktop.login1.Manager"));
        assert!(s.contains("PrepareForSleep"));
    }
}
