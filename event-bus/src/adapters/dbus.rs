//! D-Bus adapter: native zbus subscriptions for notifications and login events.

use super::publish_event;
use futures::StreamExt;
use serde_json::json;
use tracing::{info, warn};
use zbus::message::Type;
use zbus::{Connection, MatchRule, Message, MessageStream};

const NOTIFY_INTERFACE: &str = "org.freedesktop.Notifications";
const NOTIFY_MEMBER: &str = "Notify";
const LOGIN_INTERFACE: &str = "org.freedesktop.login1.Manager";
const LOGIN_MEMBER: &str = "PrepareForSleep";

pub async fn run() {
    if std::env::var("THE_MACHINE_DISABLE_DBUS").is_ok() {
        info!("dbus adapter disabled");
        return;
    }
    loop {
        match run_adapter().await {
            Ok(()) => warn!("dbus adapter stream ended; restarting in 5s"),
            Err(e) => warn!("dbus adapter unavailable: {}; retry in 30s", e),
        }
        tokio::time::sleep(std::time::Duration::from_secs(if cfg!(test) { 1 } else { 30 })).await;
    }
}

async fn run_adapter() -> Result<(), String> {
    let conn = Connection::system()
        .await
        .map_err(|e| format!("system bus connect failed: {e}"))?;

    let mut streams = Vec::new();
    for rule in dbus_match_rules()? {
        let stream = MessageStream::for_match_rule(rule, &conn, None)
            .await
            .map_err(|e| format!("for_match_rule failed: {e}"))?;
        streams.push(stream);
    }

    let mut stream = futures::stream::select_all(streams);
    info!("zbus dbus adapter started (system bus)");
    while let Some(msg) = stream.next().await {
        let msg = msg.map_err(|e| e.to_string())?;
        if let Some((category, pattern, payload)) = signal_to_event(&msg) {
            publish_event(category, pattern, payload).await;
        }
    }
    Ok(())
}

fn dbus_match_rules() -> Result<Vec<MatchRule<'static>>, String> {
    let notify = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(NOTIFY_INTERFACE)
        .map_err(|e| e.to_string())?
        .member(NOTIFY_MEMBER)
        .map_err(|e| e.to_string())?
        .build();
    let login = MatchRule::builder()
        .msg_type(Type::Signal)
        .interface(LOGIN_INTERFACE)
        .map_err(|e| e.to_string())?
        .member(LOGIN_MEMBER)
        .map_err(|e| e.to_string())?
        .build();
    Ok(vec![notify, login])
}

fn signal_to_event(msg: &Message) -> Option<(&'static str, &'static str, serde_json::Value)> {
    let header = msg.header();
    let interface = header.interface()?.as_str();
    let member = header.member()?.as_str();
    match (interface, member) {
        (NOTIFY_INTERFACE, NOTIFY_MEMBER) => Some(parse_notify(msg)),
        (LOGIN_INTERFACE, LOGIN_MEMBER) => Some(parse_prepare_for_sleep(msg)),
        _ => None,
    }
}

fn parse_notify(msg: &Message) -> (&'static str, &'static str, serde_json::Value) {
    let body = msg.body();
    let (summary, body_text) = body
        .deserialize_unchecked::<(String, u32, String, String, String)>()
        .map(|(_, _, _, summary, body_text)| (summary, body_text))
        .unwrap_or_default();
    let payload = json!({
        "summary": summary,
        "body": body_text,
        "interface": NOTIFY_INTERFACE,
        "member": NOTIFY_MEMBER,
    });
    ("notification", "desktop.notify", payload)
}

fn parse_prepare_for_sleep(msg: &Message) -> (&'static str, &'static str, serde_json::Value) {
    let starting = msg
        .body()
        .deserialize_unchecked::<bool>()
        .unwrap_or(false);
    let payload = json!({
        "starting": starting,
        "interface": LOGIN_INTERFACE,
        "member": LOGIN_MEMBER,
    });
    ("system", "login.prepare_sleep", payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::Message;

    #[test]
    fn match_rules_cover_notify_and_login() {
        let rules = dbus_match_rules().expect("rules");
        assert_eq!(rules.len(), 2);
        let ifaces: Vec<_> = rules
            .iter()
            .filter_map(|r| r.interface().map(|i| i.as_str()))
            .collect();
        assert!(ifaces.contains(&NOTIFY_INTERFACE));
        assert!(ifaces.contains(&LOGIN_INTERFACE));
    }

    #[test]
    fn parse_notify_extracts_summary() {
        let msg = Message::signal(
            "/org/freedesktop/Notifications",
            NOTIFY_INTERFACE,
            NOTIFY_MEMBER,
        )
        .unwrap()
        .build(&(
            "app",
            0u32,
            "icon",
            "Hello",
            "Body",
            &[] as &[&str],
            std::collections::HashMap::<&str, zbus::zvariant::Value>::new(),
            0i32,
        ))
        .unwrap();
        let (category, pattern, payload) = signal_to_event(&msg).expect("event");
        assert_eq!(category, "notification");
        assert_eq!(pattern, "desktop.notify");
        assert_eq!(payload["summary"], "Hello");
    }

    #[test]
    fn parse_prepare_for_sleep_reads_bool() {
        let msg = Message::signal(
            "/org/freedesktop/login1",
            LOGIN_INTERFACE,
            LOGIN_MEMBER,
        )
        .unwrap()
        .build(&true)
        .unwrap();
        let (category, pattern, payload) = signal_to_event(&msg).expect("event");
        assert_eq!(category, "system");
        assert_eq!(pattern, "login.prepare_sleep");
        assert_eq!(payload["starting"], true);
    }
}
