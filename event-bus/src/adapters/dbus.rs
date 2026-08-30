//! D-Bus adapter: native zbus subscriptions for desktop notifications and login events.

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
use tracing::{info, warn};
use zbus::match_rule::MatchRule;
use zbus::message::Type;
use zbus::{Connection, Message, MessageStream};

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match listen_signals().await {
            Ok(()) => warn!("dbus connection closed; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        let delay = if cfg!(test) { 1 } else { 30 };
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
    }
}

async fn listen_signals() -> Result<(), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("system bus connect failed: {e}"))?;

    let notify_rule = match_rule_notify().map_err(|e| e.to_string())?;
    let sleep_rule = match_rule_prepare_sleep().map_err(|e| e.to_string())?;

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(16))
        .await
        .map_err(|e| format!("notify match rule failed: {e}"))?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(4))
        .await
        .map_err(|e| format!("PrepareForSleep match rule failed: {e}"))?;

    info!("dbus zbus adapter started");

    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(Ok(message)) => handle_notify(message).await,
                    Some(Err(e)) => return Err(format!("notify stream error: {e}")),
                    None => break,
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(Ok(message)) => handle_prepare_sleep(message).await,
                    Some(Err(e)) => return Err(format!("PrepareForSleep stream error: {e}")),
                    None => break,
                }
            }
        }
    }
    Ok(())
}

fn match_rule_notify() -> zbus::Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.Notifications")?
        .member("Notify")?
        .build())
}

fn match_rule_prepare_sleep() -> zbus::Result<MatchRule<'static>> {
    Ok(MatchRule::builder()
        .msg_type(Type::Signal)
        .interface("org.freedesktop.login1.Manager")?
        .member("PrepareForSleep")?
        .build())
}

async fn handle_notify(msg: Message) {
    let payload = match decode_notify(&msg) {
        Ok(payload) => payload,
        Err(e) => {
            warn!("Notify decode failed: {e}");
            json!({ "member": "Notify", "decode_error": e.to_string() })
        }
    };
    publish_event("notification", "desktop.notify", payload).await;
}

async fn handle_prepare_sleep(msg: Message) {
    let payload = match decode_prepare_sleep(&msg) {
        Ok(payload) => payload,
        Err(e) => {
            warn!("PrepareForSleep decode failed: {e}");
            json!({ "member": "PrepareForSleep", "decode_error": e.to_string() })
        }
    };
    publish_event("system", "login.prepare_sleep", payload).await;
}

fn decode_notify(msg: &Message) -> zbus::Result<Value> {
    let body = msg.body();
    let (
        app_name,
        replaces_id,
        app_icon,
        summary,
        body_text,
        _actions,
        _hints,
        expire_timeout,
    ): (
        String,
        u32,
        String,
        String,
        String,
        zbus::zvariant::Value<'_>,
        zbus::zvariant::Value<'_>,
        i32,
    ) = body.deserialize()?;
    Ok(build_notify_payload(
        &app_name,
        replaces_id,
        &app_icon,
        &summary,
        &body_text,
        expire_timeout,
    ))
}

fn decode_prepare_sleep(msg: &Message) -> zbus::Result<Value> {
    let body = msg.body();
    let start: bool = body.deserialize()?;
    Ok(build_prepare_sleep_payload(start))
}

fn build_notify_payload(
    app_name: &str,
    replaces_id: u32,
    app_icon: &str,
    summary: &str,
    body: &str,
    expire_timeout: i32,
) -> Value {
    json!({
        "app_name": app_name,
        "replaces_id": replaces_id,
        "app_icon": app_icon,
        "summary": summary,
        "body": body,
        "expire_timeout": expire_timeout,
    })
}

fn build_prepare_sleep_payload(start: bool) -> Value {
    json!({ "start": start })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_match_rule_targets_freedesktop_notifications() {
        let rule = match_rule_notify().expect("notify match rule");
        assert!(rule.to_string().contains("org.freedesktop.Notifications"));
        assert!(rule.to_string().contains("Notify"));
    }

    #[test]
    fn prepare_sleep_match_rule_targets_login_manager() {
        let rule = match_rule_prepare_sleep().expect("sleep match rule");
        assert!(rule.to_string().contains("org.freedesktop.login1.Manager"));
        assert!(rule.to_string().contains("PrepareForSleep"));
    }

    #[test]
    fn notify_payload_includes_summary_and_body() {
        let payload = build_notify_payload("app", 1, "icon", "Title", "Body text", 5000);
        assert_eq!(payload["summary"], "Title");
        assert_eq!(payload["body"], "Body text");
        assert_eq!(payload["app_name"], "app");
    }

    #[test]
    fn prepare_sleep_payload_carries_start_flag() {
        let payload = build_prepare_sleep_payload(true);
        assert_eq!(payload["start"], true);
    }
}
