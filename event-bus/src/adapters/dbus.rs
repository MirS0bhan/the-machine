//! D-Bus adapter: native system-bus signal subscription via `zbus`.

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
        match run_subscriptions().await {
            Ok(()) => warn!("dbus subscription ended; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        let delay = if cfg!(test) { 1 } else { 30 };
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
    }
}

async fn run_subscriptions() -> Result<(), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("system bus connect: {e}"))?;

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

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(8))
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(4))
        .await
        .map_err(|e| e.to_string())?;

    info!("dbus system-bus subscriptions active (Notifications, login1.PrepareForSleep)");

    loop {
        tokio::select! {
            msg = notify_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_notify(msg).await,
                    Some(Err(e)) => return Err(format!("notify stream: {e}")),
                    None => return Ok(()),
                }
            }
            msg = sleep_stream.next() => {
                match msg {
                    Some(Ok(msg)) => handle_prepare_sleep(msg).await,
                    Some(Err(e)) => return Err(format!("sleep stream: {e}")),
                    None => return Ok(()),
                }
            }
        }
    }
}

async fn handle_notify(msg: Message) {
    let payload = match notify_payload(&msg) {
        Ok(p) => p,
        Err(e) => {
            warn!("failed to parse Notify signal: {e}");
            json!({ "parse_error": e.to_string() })
        }
    };
    publish_event("notification", "desktop.notify", payload).await;
}

async fn handle_prepare_sleep(msg: Message) {
    let payload = match prepare_sleep_payload(&msg) {
        Ok(p) => p,
        Err(e) => {
            warn!("failed to parse PrepareForSleep signal: {e}");
            json!({ "parse_error": e.to_string() })
        }
    };
    publish_event("system", "login.prepare_sleep", payload).await;
}

fn notify_payload(msg: &Message) -> Result<Value, zbus::Error> {
    let (app_name, _replaces_id, app_icon, summary, body, _actions, _hints, _expire): (
        String,
        u32,
        String,
        String,
        String,
        Vec<String>,
        std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        i32,
    ) = msg.body().deserialize()?;

    Ok(json!({
        "app_name": app_name,
        "app_icon": app_icon,
        "summary": summary,
        "body": body,
    }))
}

fn prepare_sleep_payload(msg: &Message) -> Result<Value, zbus::Error> {
    let start: bool = msg.body().deserialize()?;
    Ok(json!({ "start": start }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::Message;

    fn notify_message(
        app_name: &str,
        summary: &str,
        body: &str,
    ) -> zbus::Result<Message> {
        Message::signal(
            "/org/freedesktop/Notifications",
            "org.freedesktop.Notifications",
            "Notify",
        )?
        .build(&(
            app_name,
            0u32,
            "",
            summary,
            body,
            Vec::<String>::new(),
            std::collections::HashMap::<String, zbus::zvariant::OwnedValue>::new(),
            0i32,
        ))
    }

    fn prepare_sleep_message(start: bool) -> zbus::Result<Message> {
        Message::signal(
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
            "PrepareForSleep",
        )?
        .build(&(start,))
    }

    #[test]
    fn notify_payload_parses_summary_and_body() {
        let msg = notify_message("demo", "Title", "Details").unwrap();
        let payload = notify_payload(&msg).unwrap();
        assert_eq!(payload["app_name"], "demo");
        assert_eq!(payload["summary"], "Title");
        assert_eq!(payload["body"], "Details");
    }

    #[test]
    fn prepare_sleep_payload_parses_start_flag() {
        let msg = prepare_sleep_message(true).unwrap();
        let payload = prepare_sleep_payload(&msg).unwrap();
        assert_eq!(payload["start"], true);
    }
}
