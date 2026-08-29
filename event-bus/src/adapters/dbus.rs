//! D-Bus adapter: monitors notifications and login events via native zbus.

use super::publish_event;
use futures::StreamExt;
use serde_json::json;
use tracing::{info, warn};
use zbus::{message::Type, Connection, MatchRule, Message, MessageStream};

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
        let delay_secs = if cfg!(test) { 1 } else { 30 };
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
    }
}

async fn monitor_signals() -> Result<(), String> {
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

async fn handle_notify(msg: Message) {
    let summary = notify_summary(&msg);
    publish_event(
        "notification",
        "desktop.notify",
        json!({ "raw": format!("{msg:?}"), "summary": summary }),
    )
    .await;
}

async fn handle_prepare_sleep(msg: Message) {
    let active = msg.body().deserialize::<bool>().unwrap_or(false);
    publish_event(
        "system",
        "login.prepare_sleep",
        json!({ "raw": format!("{msg:?}"), "active": active }),
    )
    .await;
}

/// Extract the notification summary (4th Notify arg) when present.
fn notify_summary(msg: &Message) -> String {
    msg.body()
        .deserialize::<(String, u32, String, String, String)>()
        .ok()
        .map(|(_, _, _, summary, _)| summary)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::notify_summary;
    use zbus::Message;

    #[test]
    fn notify_summary_empty_for_non_notify_body() {
        let msg = Message::signal("/", "org.test.Interface", "Other")
            .unwrap()
            .build(&())
            .unwrap();
        assert!(notify_summary(&msg).is_empty());
    }
}
