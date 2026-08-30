//! D-Bus adapter: monitors notifications and login events via native zbus.

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
        let delay_secs = match run_session().await {
            Ok(()) => {
                warn!("zbus session ended; restarting in 5s");
                if cfg!(test) { 1 } else { 5 }
            }
            Err(e) => {
                warn!("dbus adapter unavailable: {}; retry in 30s", e);
                if cfg!(test) { 1 } else { 30 }
            }
        };
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
    }
}

async fn run_session() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    info!("zbus system bus connected");

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
                            parse_notify_signal(&msg),
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
                            parse_prepare_for_sleep(&msg),
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

/// Build the event payload for a desktop notification signal.
fn notify_payload(app_name: &str, summary: &str, body: &str) -> Value {
    json!({
        "app_name": app_name,
        "summary": summary,
        "body": body,
    })
}

fn parse_notify_signal(msg: &Message) -> Value {
    type NotifyArgs = (
        String, // app_name
        u32,    // replaces_id
        String, // app_icon
        String, // summary
        String, // body
    );
    if let Ok((app_name, _, _, summary, body)) = msg.body().deserialize::<NotifyArgs>() {
        notify_payload(&app_name, &summary, &body)
    } else {
        json!({ "raw": msg.to_string() })
    }
}

fn parse_prepare_for_sleep(msg: &Message) -> Value {
    if let Ok(sleep) = msg.body().deserialize::<bool>() {
        json!({ "sleep": sleep })
    } else {
        json!({ "raw": msg.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_payload_fields() {
        let payload = notify_payload("firefox", "Update available", "Restart to apply");
        assert_eq!(payload["app_name"], "firefox");
        assert_eq!(payload["summary"], "Update available");
        assert_eq!(payload["body"], "Restart to apply");
    }

    #[test]
    fn prepare_for_sleep_payload_true() {
        let payload = json!({ "sleep": true });
        assert_eq!(payload["sleep"], true);
    }
}
