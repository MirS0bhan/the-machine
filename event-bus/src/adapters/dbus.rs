//! D-Bus adapter: native zbus subscriptions for desktop notifications and login events.

use super::publish_event;
use futures::StreamExt;
use serde_json::{json, Value};
use tracing::{info, warn};
use zbus::{match_rule::MatchRule, Connection, Message, MessageStream};

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match listen().await {
            Ok(()) => warn!("dbus connection closed; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        let delay = if cfg!(test) { 1 } else { 30 };
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
    }
}

const NOTIFY_MATCH: &str =
    "type='signal',interface='org.freedesktop.Notifications',member='Notify'";
const SLEEP_MATCH: &str =
    "type='signal',interface='org.freedesktop.login1.Manager',member='PrepareForSleep'";

async fn listen() -> Result<(), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    info!("dbus adapter connected (zbus)");

    let notify_rule = MatchRule::try_from(NOTIFY_MATCH).map_err(|e| e.to_string())?;
    let sleep_rule = MatchRule::try_from(SLEEP_MATCH).map_err(|e| e.to_string())?;

    let mut notify_stream = MessageStream::for_match_rule(notify_rule, &conn, Some(16))
        .await
        .map_err(|e| e.to_string())?;
    let mut sleep_stream = MessageStream::for_match_rule(sleep_rule, &conn, Some(16))
        .await
        .map_err(|e| e.to_string())?;

    loop {
        tokio::select! {
            msg = notify_stream.next() => match msg {
                Some(Ok(msg)) => publish_event("notification", "desktop.notify", notify_payload(&msg)).await,
                Some(Err(e)) => return Err(e.to_string()),
                None => return Ok(()),
            },
            msg = sleep_stream.next() => match msg {
                Some(Ok(msg)) => publish_event("system", "login.prepare_sleep", sleep_payload(&msg)).await,
                Some(Err(e)) => return Err(e.to_string()),
                None => return Ok(()),
            },
        }
    }
}

fn notify_payload(msg: &Message) -> Value {
    let raw = format!("{msg:?}");
    let summary = decode_notify_summary(msg).unwrap_or_default();
    json!({ "raw": raw, "summary": summary })
}

fn sleep_payload(msg: &Message) -> Value {
    let raw = format!("{msg:?}");
    let start = msg.body().deserialize::<bool>().ok();
    json!({ "raw": raw, "start": start })
}

fn decode_notify_summary(msg: &Message) -> Option<String> {
    type NotifyArgs = (
        String,
        u32,
        String,
        String,
        String,
        Vec<String>,
        std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
        i32,
    );
    msg.body()
        .deserialize::<NotifyArgs>()
        .ok()
        .map(|(_, _, _, summary, _, _, _, _)| summary)
}

#[cfg(test)]
mod tests {
    use super::{NOTIFY_MATCH, SLEEP_MATCH};
    use zbus::match_rule::MatchRule;

    #[test]
    fn notify_match_rule_matches_dbus_monitor_filter() {
        let rule = MatchRule::try_from(NOTIFY_MATCH).unwrap().to_string();
        assert_eq!(rule, NOTIFY_MATCH);
    }

    #[test]
    fn sleep_match_rule_matches_dbus_monitor_filter() {
        let rule = MatchRule::try_from(SLEEP_MATCH).unwrap().to_string();
        assert_eq!(rule, SLEEP_MATCH);
    }
}
