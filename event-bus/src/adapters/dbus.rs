//! D-Bus adapter: monitors notifications and login events via zbus (no host `dbus-monitor`).

use super::publish_event;
use futures::StreamExt;
use serde_json::json;
use std::collections::HashMap;
use tracing::{info, warn};
use zbus::message::Type;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, MatchRule, Message, MessageStream};

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match monitor_signals().await {
            Ok(()) => warn!("dbus connection closed; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
    }
}

async fn monitor_signals() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    info!("dbus system connection established");

    let notify_rule = match_rule("org.freedesktop.Notifications", "Notify")?;
    let sleep_rule = match_rule("org.freedesktop.login1.Manager", "PrepareForSleep")?;

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

fn match_rule<'a>(interface: &'a str, member: &'a str) -> Result<MatchRule<'a>, String> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(interface)
        .map_err(|e| e.to_string())?
        .member(member)
        .map_err(|e| e.to_string())?
        .build())
}

async fn handle_notify(msg: Message) {
    publish_event("notification", "desktop.notify", notify_payload(&msg)).await;
}

async fn handle_prepare_sleep(msg: Message) {
    publish_event(
        "system",
        "login.prepare_sleep",
        prepare_sleep_payload(&msg),
    )
    .await;
}

fn notify_payload(msg: &Message) -> serde_json::Value {
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
    let body = msg.body();
    if let Ok(parsed) = body.deserialize::<NotifyBody>() {
        json!({
            "app_name": parsed.0,
            "replaces_id": parsed.1,
            "app_icon": parsed.2,
            "summary": parsed.3,
            "body": parsed.4,
            "expire_timeout": parsed.7,
        })
    } else {
        json!({ "raw": format!("{msg:?}") })
    }
}

fn prepare_sleep_payload(msg: &Message) -> serde_json::Value {
    let body = msg.body();
    if let Ok((start,)) = body.deserialize::<(bool,)>() {
        json!({ "start": start })
    } else {
        json!({ "raw": format!("{msg:?}") })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal_message<B: serde::Serialize + zbus::zvariant::DynamicType>(
        interface: &str,
        member: &str,
        body: &B,
    ) -> Message {
        Message::signal("/org/test", interface, member)
            .unwrap()
            .build(body)
            .unwrap()
    }

    #[test]
    fn notify_payload_parses_structured_body() {
        let body = (
            "app",
            0u32,
            "icon",
            "Summary",
            "Body text",
            Vec::<String>::new(),
            HashMap::<String, OwnedValue>::new(),
            -1i32,
        );
        let msg = signal_message("org.freedesktop.Notifications", "Notify", &body);
        let payload = notify_payload(&msg);
        assert_eq!(payload["summary"], "Summary");
        assert_eq!(payload["body"], "Body text");
        assert_eq!(payload["app_name"], "app");
    }

    #[test]
    fn prepare_sleep_payload_parses_bool() {
        let body = (true,);
        let msg = signal_message("org.freedesktop.login1.Manager", "PrepareForSleep", &body);
        let payload = prepare_sleep_payload(&msg);
        assert_eq!(payload["start"], true);
    }

    #[test]
    fn match_rules_match_dbus_monitor_filters() {
        let notify = match_rule("org.freedesktop.Notifications", "Notify").unwrap();
        assert!(notify.to_string().contains("org.freedesktop.Notifications"));
        assert!(notify.to_string().contains("Notify"));

        let sleep = match_rule("org.freedesktop.login1.Manager", "PrepareForSleep").unwrap();
        assert!(sleep.to_string().contains("org.freedesktop.login1.Manager"));
        assert!(sleep.to_string().contains("PrepareForSleep"));
    }
}
