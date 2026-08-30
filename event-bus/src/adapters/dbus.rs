//! D-Bus adapter: native zbus subscriptions for desktop notifications and login events.

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
        match listen_signals().await {
            Ok(()) => warn!("dbus adapter ended; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
    }
}

async fn listen_signals() -> Result<(), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("system bus connect failed: {e}"))?;
    info!("dbus adapter connected to system bus");

    let notify_rule = match_rule(
        "org.freedesktop.Notifications",
        "Notify",
    )?;
    let sleep_rule = match_rule(
        "org.freedesktop.login1.Manager",
        "PrepareForSleep",
    )?;

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(16))
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(4))
        .await
        .map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_notify(&msg).await,
                    Some(Err(e)) => return Err(format!("notify stream: {e}")),
                    None => return Err("notify stream closed".into()),
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_prepare_sleep(&msg).await,
                    Some(Err(e)) => return Err(format!("sleep stream: {e}")),
                    None => return Err("sleep stream closed".into()),
                }
            }
        }
    }
}

fn match_rule(interface: &str, member: &str) -> Result<MatchRule<'static>, String> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(interface.to_string())
        .map_err(|e| e.to_string())?
        .member(member.to_string())
        .map_err(|e| e.to_string())?
        .build())
}

async fn handle_notify(msg: &Message) {
    let payload = notify_payload(msg);
    publish_event("notification", "desktop.notify", payload).await;
}

async fn handle_prepare_sleep(msg: &Message) {
    let payload = prepare_sleep_payload(msg);
    publish_event("system", "login.prepare_sleep", payload).await;
}

/// Build a normalized payload for `org.freedesktop.Notifications.Notify`.
fn notify_payload(msg: &Message) -> Value {
    type NotifyArgs = (
        String,
        u32,
        String,
        String,
        String,
        Vec<String>,
        std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        i32,
    );

    let body = msg.body();
    match body.deserialize::<NotifyArgs>() {
        Ok((app_name, replaces_id, app_icon, summary, body, actions, hints, expire_timeout)) => {
            json!({
                "app_name": app_name,
                "replaces_id": replaces_id,
                "app_icon": app_icon,
                "summary": summary,
                "body": body,
                "actions": actions,
                "hints": hints,
                "expire_timeout": expire_timeout,
            })
        }
        Err(e) => json!({
            "parse_error": e.to_string(),
            "summary": extract_field(&format!("{msg:?}"), "string"),
        }),
    }
}

/// Build a normalized payload for `org.freedesktop.login1.Manager.PrepareForSleep`.
fn prepare_sleep_payload(msg: &Message) -> Value {
    match msg.body().deserialize::<bool>() {
        Ok(sleep) => json!({ "sleep": sleep }),
        Err(e) => json!({ "parse_error": e.to_string() }),
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
    fn prepare_sleep_payload_from_bool_body() {
        let msg = Message::signal("/org/freedesktop/login1", "org.freedesktop.login1.Manager", "PrepareForSleep")
            .unwrap()
            .build(&true)
            .unwrap();
        let payload = prepare_sleep_payload(&msg);
        assert_eq!(payload.get("sleep").and_then(|v| v.as_bool()), Some(true));
    }

    #[test]
    fn notify_payload_extracts_summary() {
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
            "Test summary",
            "Test body",
            Vec::<String>::new(),
            std::collections::HashMap::<String, zbus::zvariant::OwnedValue>::new(),
            -1i32,
        ))
        .unwrap();
        let payload = notify_payload(&msg);
        assert_eq!(
            payload.get("summary").and_then(|v| v.as_str()),
            Some("Test summary")
        );
        assert_eq!(
            payload.get("body").and_then(|v| v.as_str()),
            Some("Test body")
        );
    }
}
