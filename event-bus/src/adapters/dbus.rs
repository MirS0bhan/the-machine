//! D-Bus adapter: native zbus signal subscriptions (no external `dbus-monitor`).

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value as JsonValue};
use tracing::{info, warn};
use zbus::match_rule::MatchRule;
use zbus::message::Type;
use zbus::zvariant::OwnedValue;
use zbus::{Connection, Message, MessageStream};

const RETRY_UNAVAILABLE_SECS: u64 = 30;
const RETRY_EXITED_SECS: u64 = 5;

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match monitor_signals().await {
            Ok(()) => {
                warn!(
                    "dbus signal stream ended; restarting in {}s",
                    RETRY_EXITED_SECS
                );
                tokio::time::sleep(std::time::Duration::from_secs(RETRY_EXITED_SECS)).await;
            }
            Err(e) => {
                warn!(
                    "dbus adapter unavailable: {}; retry in {}s",
                    e, RETRY_UNAVAILABLE_SECS
                );
                let delay = if cfg!(test) { 1 } else { RETRY_UNAVAILABLE_SECS };
                tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
            }
        }
    }
}

async fn monitor_signals() -> Result<(), String> {
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

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, None)
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, None)
        .await
        .map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(Ok(msg)) => {
                        publish_event(
                            "notification",
                            "desktop.notify",
                            notify_payload(&msg),
                        )
                        .await;
                    }
                    Some(Err(e)) => return Err(e.to_string()),
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
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
        }
    }
}

fn notify_payload(msg: &Message) -> JsonValue {
    type NotifyArgs = (
        String,
        u32,
        String,
        String,
        String,
        Vec<String>,
        std::collections::HashMap<String, OwnedValue>,
        i32,
    );

    match msg.body().deserialize::<NotifyArgs>() {
        Ok((app_name, replaces_id, app_icon, summary, body, actions, _hints, expire_timeout)) => {
            notification_payload_from_parts(
                &app_name,
                &summary,
                &body,
                &app_icon,
                replaces_id,
                actions,
                expire_timeout,
            )
        }
        Err(e) => json!({
            "member": "Notify",
            "parse_error": e.to_string(),
        }),
    }
}

fn prepare_sleep_payload(msg: &Message) -> JsonValue {
    match msg.body().deserialize::<bool>() {
        Ok(start) => prepare_sleep_payload_from_start(start),
        Err(e) => json!({
            "member": "PrepareForSleep",
            "parse_error": e.to_string(),
        }),
    }
}

fn notification_payload_from_parts(
    app_name: &str,
    summary: &str,
    body: &str,
    app_icon: &str,
    replaces_id: u32,
    actions: Vec<String>,
    expire_timeout: i32,
) -> JsonValue {
    json!({
        "app_name": app_name,
        "summary": summary,
        "body": body,
        "app_icon": app_icon,
        "replaces_id": replaces_id,
        "actions": actions,
        "expire_timeout": expire_timeout,
    })
}

fn prepare_sleep_payload_from_start(start: bool) -> JsonValue {
    json!({ "start": start })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_payload_includes_summary() {
        let payload = notification_payload_from_parts(
            "app",
            "Hello",
            "World",
            "icon",
            0,
            vec!["act".into()],
            5000,
        );
        assert_eq!(payload["summary"], "Hello");
        assert_eq!(payload["body"], "World");
        assert_eq!(payload["app_name"], "app");
    }

    #[test]
    fn prepare_sleep_payload_marks_sleep_phase() {
        let sleeping = prepare_sleep_payload_from_start(true);
        assert_eq!(sleeping["start"], true);
        let waking = prepare_sleep_payload_from_start(false);
        assert_eq!(waking["start"], false);
    }
}
