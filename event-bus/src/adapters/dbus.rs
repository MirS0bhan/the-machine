//! D-Bus adapter: subscribes to desktop notifications and login events via zbus.

use super::publish_event;
use futures_util::StreamExt;
use serde_json::json;
use tracing::{info, warn};
use zbus::message::Type;
use zbus::{Connection, MatchRule, Message, MessageStream};

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match run_subscriber().await {
            Ok(()) => warn!("dbus subscriber exited; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
    }
}

async fn run_subscriber() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    info!("dbus adapter connected to system bus");

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
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(8))
        .await
        .map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            msg = notify_stream.next() => match msg {
                Some(Ok(msg)) => handle_notify(msg).await,
                Some(Err(e)) => return Err(e.to_string()),
                None => return Ok(()),
            },
            msg = sleep_stream.next() => match msg {
                Some(Ok(msg)) => handle_prepare_for_sleep(msg).await,
                Some(Err(e)) => return Err(e.to_string()),
                None => return Ok(()),
            },
        }
    }
}

async fn handle_notify(msg: Message) {
    let payload = notify_payload(&msg);
    publish_event("notification", "desktop.notify", payload).await;
}

async fn handle_prepare_for_sleep(msg: Message) {
    let payload = prepare_for_sleep_payload(&msg);
    publish_event("system", "login.prepare_sleep", payload).await;
}

/// Build a normalized payload for `org.freedesktop.Notifications.Notify`.
fn notify_payload(msg: &Message) -> serde_json::Value {
    let summary = parse_notify_summary(msg).unwrap_or_default();
    json!({
        "summary": summary,
        "interface": "org.freedesktop.Notifications",
        "member": "Notify",
    })
}

/// Build a normalized payload for `org.freedesktop.login1.Manager.PrepareForSleep`.
fn prepare_for_sleep_payload(msg: &Message) -> serde_json::Value {
    let start = msg
        .body()
        .deserialize::<bool>()
        .unwrap_or(false);
    json!({
        "prepare_for_sleep": start,
        "interface": "org.freedesktop.login1.Manager",
        "member": "PrepareForSleep",
    })
}

/// Parse the summary field from a Notifications.Notify signal body.
fn parse_notify_summary(msg: &Message) -> Option<String> {
    let body = msg.body();
    let (_app_name, _replaces_id, _app_icon, summary, _body): (
        u32,
        u32,
        String,
        String,
        String,
    ) = body.deserialize().ok()?;
    Some(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_for_sleep_payload_defaults_when_body_missing() {
        let msg = Message::signal(
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
            "PrepareForSleep",
        )
        .unwrap()
        .build(&())
        .unwrap();
        let payload = prepare_for_sleep_payload(&msg);
        assert_eq!(payload["prepare_for_sleep"], false);
        assert_eq!(payload["member"], "PrepareForSleep");
    }

    #[test]
    fn prepare_for_sleep_payload_reads_boolean_body() {
        let msg = Message::signal(
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
            "PrepareForSleep",
        )
        .unwrap()
        .build(&true)
        .unwrap();
        let payload = prepare_for_sleep_payload(&msg);
        assert_eq!(payload["prepare_for_sleep"], true);
    }

    #[test]
    fn notify_payload_includes_summary_when_present() {
        let msg = Message::signal(
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
        )
        .unwrap()
        .build(&(0u32, 0u32, "app", "Battery low", "Plug in charger"))
        .unwrap();
        let payload = notify_payload(&msg);
        assert_eq!(payload["summary"], "Battery low");
        assert_eq!(payload["member"], "Notify");
    }

    #[test]
    fn parse_notify_summary_extracts_fourth_string() {
        let msg = Message::signal(
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
        )
        .unwrap()
        .build(&(1u32, 2u32, "icon", "Hello", "World"))
        .unwrap();
        assert_eq!(parse_notify_summary(&msg).as_deref(), Some("Hello"));
    }
}
