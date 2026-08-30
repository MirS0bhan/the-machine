//! D-Bus adapter: monitors desktop notifications and sleep events via native zbus.

use super::publish_event;
use futures::StreamExt;
use serde_json::json;
use tracing::{info, warn};
use zbus::match_rule::MatchRule;
use zbus::{Connection, Message, MessageStream};

const NOTIFY_RULE: &str =
    "type='signal',interface='org.freedesktop.Notifications',member='Notify'";
const SLEEP_RULE: &str =
    "type='signal',interface='org.freedesktop.login1.Manager',member='PrepareForSleep'";

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
        let delay = if cfg!(test) { 1 } else { 30 };
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
    }
}

async fn monitor_signals() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    info!("dbus system bus connected");

    let notify_rule = MatchRule::try_from(NOTIFY_RULE).map_err(|e| e.to_string())?;
    let sleep_rule = MatchRule::try_from(SLEEP_RULE).map_err(|e| e.to_string())?;

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
                    Some(Ok(msg)) => handle_prepare_for_sleep(msg).await,
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
        }
    }
}

async fn handle_notify(msg: Message) {
    let summary = notify_summary(&msg).unwrap_or_default();
    let payload = json!({
        "summary": summary,
        "interface": msg.header().interface().map(|i| i.as_str()),
        "member": msg.header().member().map(|m| m.as_str()),
    });
    publish_event("notification", "desktop.notify", payload).await;
}

async fn handle_prepare_for_sleep(msg: Message) {
    let active = prepare_for_sleep_active(&msg);
    let payload = json!({
        "active": active,
        "interface": msg.header().interface().map(|i| i.as_str()),
        "member": msg.header().member().map(|m| m.as_str()),
    });
    publish_event("system", "login.prepare_sleep", payload).await;
}

/// Extract the notification summary from a `Notify` signal body (`susssasa{sv}i`).
fn notify_summary(msg: &Message) -> Result<String, zbus::Error> {
    let body = msg.body();
    let (_app_name, _replaces_id, _app_icon, summary, _body, _actions, _hints, _expire): (
        String,
        u32,
        String,
        String,
        String,
        Vec<String>,
        std::collections::HashMap<String, zbus::zvariant::Value<'_>>,
        i32,
    ) = body.deserialize()?;
    Ok(summary)
}

/// Extract the `active` flag from a `PrepareForSleep` signal body (`b`).
fn prepare_for_sleep_active(msg: &Message) -> Option<bool> {
    msg.body().deserialize().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_match_rules_parse() {
        assert!(MatchRule::try_from(NOTIFY_RULE).is_ok());
        assert!(MatchRule::try_from(SLEEP_RULE).is_ok());
    }

    #[test]
    fn builder_match_rules_match_legacy_strings() {
        use zbus::message::Type;

        let notify = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.Notifications")
            .unwrap()
            .member("Notify")
            .unwrap()
            .build();
        assert_eq!(notify.to_string(), NOTIFY_RULE);

        let sleep = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.login1.Manager")
            .unwrap()
            .member("PrepareForSleep")
            .unwrap()
            .build();
        assert_eq!(sleep.to_string(), SLEEP_RULE);
    }
}
