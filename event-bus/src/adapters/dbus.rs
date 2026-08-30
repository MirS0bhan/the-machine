//! D-Bus adapter: native zbus signal monitor for desktop notifications and login events.

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
use tracing::{info, warn};
use zbus::message::Type;
use zbus::{Connection, MatchRule, Message, MessageStream};

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
                    "dbus signal monitor exited; restarting in {}s",
                    RETRY_EXITED_SECS
                );
                tokio::time::sleep(std::time::Duration::from_secs(RETRY_EXITED_SECS)).await;
            }
            Err(e) => {
                warn!(
                    "dbus adapter unavailable: {}; retry in {}s",
                    e, RETRY_UNAVAILABLE_SECS
                );
                tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) {
                    1
                } else {
                    RETRY_UNAVAILABLE_SECS
                }))
                .await;
            }
        }
    }
}

async fn monitor_signals() -> Result<(), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("system bus connect failed: {e}"))?;
    info!("dbus adapter connected (zbus system bus)");

    let notify_rule = signal_rule("org.freedesktop.Notifications", "Notify")?;
    let sleep_rule = signal_rule("org.freedesktop.login1.Manager", "PrepareForSleep")?;

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(32))
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(8))
        .await
        .map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_notify(&msg).await,
                    Some(Err(e)) => return Err(format!("notify stream error: {e}")),
                    None => return Ok(()),
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_prepare_for_sleep(&msg).await,
                    Some(Err(e)) => return Err(format!("sleep stream error: {e}")),
                    None => return Ok(()),
                }
            }
        }
    }
}

fn signal_rule(interface: &str, member: &str) -> Result<MatchRule<'static>, String> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(interface.to_string())
        .map_err(|e| e.to_string())?
        .member(member.to_string())
        .map_err(|e| e.to_string())?
        .build())
}

async fn handle_notify(msg: &Message) {
    publish_event("notification", "desktop.notify", notify_payload(msg)).await;
}

async fn handle_prepare_for_sleep(msg: &Message) {
    publish_event("system", "login.prepare_sleep", prepare_sleep_payload(msg)).await;
}

fn notify_payload(msg: &Message) -> Value {
    type NotifyBody = (
        String,
        u32,
        String,
        String,
        String,
        Vec<String>,
        std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        i32,
    );
    if let Ok((app_name, _, app_icon, summary, body, actions, _, expire_timeout)) =
        msg.body().deserialize::<NotifyBody>()
    {
        return json!({
            "app_name": app_name,
            "app_icon": app_icon,
            "summary": summary,
            "body": body,
            "actions": actions,
            "expire_timeout": expire_timeout,
        });
    }
    json!({ "raw": msg.to_string() })
}

fn prepare_sleep_payload(msg: &Message) -> Value {
    if let Ok(active) = msg.body().deserialize::<bool>() {
        return json!({ "active": active });
    }
    json!({ "raw": msg.to_string() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_rules_match_expected_interfaces() {
        let notify = signal_rule("org.freedesktop.Notifications", "Notify").unwrap();
        assert!(notify.to_string().contains("org.freedesktop.Notifications"));
        assert!(notify.to_string().contains("Notify"));

        let sleep =
            signal_rule("org.freedesktop.login1.Manager", "PrepareForSleep").unwrap();
        assert!(sleep.to_string().contains("org.freedesktop.login1.Manager"));
        assert!(sleep.to_string().contains("PrepareForSleep"));
    }

    #[test]
    fn prepare_sleep_payload_parses_boolean_body() {
        let msg = Message::signal(
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
            "PrepareForSleep",
        )
        .unwrap()
        .build(&true)
        .unwrap();
        let payload = prepare_sleep_payload(&msg);
        assert_eq!(payload["active"], true);
    }
}
