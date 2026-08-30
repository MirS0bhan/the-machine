//! D-Bus adapter: monitors notifications and login events via native zbus (no dbus-monitor).

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use tracing::{info, warn};
use zbus::{Connection, MatchRule, Message, MessageStream, MessageType, OwnedMatchRule};

const NOTIFY_IFACE: &str = "org.freedesktop.Notifications";
const LOGIN_IFACE: &str = "org.freedesktop.login1.Manager";

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match listen().await {
            Ok(()) => {
                warn!("dbus monitor exited; restarting in 5s");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
            Err(e) => {
                warn!("dbus adapter unavailable: {}; retry in 30s", e);
                tokio::time::sleep(Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
            }
        }
    }
}

async fn listen() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let notify_rule = signal_match_rule(NOTIFY_IFACE, "Notify")?;
    let sleep_rule = signal_match_rule(LOGIN_IFACE, "PrepareForSleep")?;

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(16))
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(8))
        .await
        .map_err(|e| e.to_string())?;

    info!("dbus signal monitor started");
    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(Ok(msg)) => dispatch_signal(&msg).await,
                    Some(Err(e)) => warn!("dbus notify stream error: {}", e),
                    None => break,
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(Ok(msg)) => dispatch_signal(&msg).await,
                    Some(Err(e)) => warn!("dbus sleep stream error: {}", e),
                    None => break,
                }
            }
        }
    }
    Ok(())
}

async fn dispatch_signal(msg: &Message) {
    if let Some((category, pattern, payload)) = decode_signal(msg) {
        publish_event(category, pattern, payload).await;
    }
}

fn signal_match_rule(interface: &str, member: &str) -> Result<OwnedMatchRule, String> {
    Ok(MatchRule::builder()
        .msg_type(MessageType::Signal)
        .interface(interface)
        .map_err(|e| e.to_string())?
        .member(member)
        .map_err(|e| e.to_string())?
        .build()
        .into())
}

pub(crate) fn decode_signal(msg: &Message) -> Option<(&'static str, &'static str, Value)> {
    let header = msg.header().ok()?;
    let member = header.member().ok()?.as_ref()?.as_str();
    match member {
        "Notify" => {
            let payload = parse_notify_message(msg)?;
            Some(("notification", "desktop.notify", payload))
        }
        "PrepareForSleep" => {
            let payload = parse_prepare_sleep_message(msg)?;
            Some(("system", "login.prepare_sleep", payload))
        }
        _ => None,
    }
}

fn parse_notify_message(msg: &Message) -> Option<Value> {
    let (app_name, _replaces_id, _app_icon, summary, body_text): (
        String,
        u32,
        String,
        String,
        String,
    ) = msg.body().ok()?;
    Some(notify_payload(&app_name, &summary, &body_text))
}

fn parse_prepare_sleep_message(msg: &Message) -> Option<Value> {
    let start: bool = msg.body().ok()?;
    Some(prepare_sleep_payload(start))
}

pub(crate) fn notify_payload(app_name: &str, summary: &str, body: &str) -> Value {
    json!({
        "app_name": app_name,
        "summary": summary,
        "body": body,
    })
}

pub(crate) fn prepare_sleep_payload(start: bool) -> Value {
    json!({ "start": start })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_payload_includes_fields() {
        let v = notify_payload("app", "title", "details");
        assert_eq!(v["app_name"], "app");
        assert_eq!(v["summary"], "title");
        assert_eq!(v["body"], "details");
    }

    #[test]
    fn prepare_sleep_payload_start_flag() {
        let v = prepare_sleep_payload(true);
        assert_eq!(v["start"], true);
    }
}
