//! D-Bus adapter: monitors notifications and login events via native zbus subscriptions.

use super::publish_event;
use futures::StreamExt;
use serde_json::json;
use tracing::{info, warn};
use zbus::{message::Type, Connection, MatchRule, Message, MessageStream};

fn notify_rule() -> Result<MatchRule<'static>, zbus::Error> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.Notifications")?
        .member("Notify")?
        .build())
}

fn sleep_rule() -> Result<MatchRule<'static>, zbus::Error> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.login1.Manager")?
        .member("PrepareForSleep")?
        .build())
}

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
    info!("dbus system bus connected");

    let notify_rule = notify_rule().map_err(|e| e.to_string())?;
    let sleep_rule = sleep_rule().map_err(|e| e.to_string())?;

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(16))
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
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_prepare_for_sleep(&msg).await,
                    Some(Err(e)) => return Err(e.to_string()),
                    None => return Ok(()),
                }
            }
        }
    }
}

async fn handle_notify(msg: &Message) {
    let parsed = parse_notify(msg);
    publish_event(
        "notification",
        "desktop.notify",
        json!({
            "summary": parsed.summary,
            "app_name": parsed.app_name,
            "body": parsed.body,
        }),
    )
    .await;
}

async fn handle_prepare_for_sleep(msg: &Message) {
    let sleeping = parse_prepare_for_sleep(msg).unwrap_or(true);
    publish_event(
        "system",
        "login.prepare_sleep",
        json!({ "sleeping": sleeping }),
    )
    .await;
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct NotifyPayload {
    app_name: String,
    summary: String,
    body: String,
}

fn parse_notify(msg: &Message) -> NotifyPayload {
    let Ok((
        app_name,
        _replaces_id,
        _app_icon,
        summary,
        body,
        _actions,
        _hints,
        _expire_timeout,
    )) = msg.body().deserialize::<(
        String,
        u32,
        String,
        String,
        String,
        Vec<String>,
        std::collections::HashMap<String, zvariant::OwnedValue>,
        i32,
    )>() else {
        return NotifyPayload::default();
    };
    NotifyPayload {
        app_name,
        summary,
        body,
    }
}

fn parse_prepare_for_sleep(msg: &Message) -> Option<bool> {
    msg.body().deserialize::<(bool,)>().ok().map(|(sleeping,)| sleeping)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_rule_matches_notifications() {
        let rule = notify_rule().expect("notify rule");
        assert_eq!(rule.msg_type(), Some(Type::Signal));
        assert_eq!(
            rule.interface().map(|i| i.as_str()),
            Some("org.freedesktop.Notifications")
        );
        assert_eq!(rule.member().map(|m| m.as_str()), Some("Notify"));
    }

    #[test]
    fn sleep_rule_matches_login_manager() {
        let rule = sleep_rule().expect("sleep rule");
        assert_eq!(rule.msg_type(), Some(Type::Signal));
        assert_eq!(
            rule.interface().map(|i| i.as_str()),
            Some("org.freedesktop.login1.Manager")
        );
        assert_eq!(rule.member().map(|m| m.as_str()), Some("PrepareForSleep"));
    }
}
