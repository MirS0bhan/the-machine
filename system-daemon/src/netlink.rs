//! Network interface admin via rtnetlink (RTM_GETLINK / RTM_SETLINK).

use common::NetworkInterface;
use futures::stream::TryStreamExt;
use netlink_packet_route::link::{LinkAttribute, LinkFlag, LinkMessage};
use rtnetlink::{new_connection, Handle};

use crate::net::{classify_iface, reject_loopback_mutation};

fn operstate_from_flags(flags: &[LinkFlag]) -> String {
    if flags.contains(&LinkFlag::Up) {
        "up".to_string()
    } else {
        "down".to_string()
    }
}

fn ifname_from_msg(msg: &LinkMessage) -> Option<String> {
    msg.attributes.iter().find_map(|attr| {
        if let LinkAttribute::IfName(name) = attr {
            Some(name.clone())
        } else {
            None
        }
    })
}

async fn netlink_handle() -> Result<Handle, String> {
    let (connection, handle, _) = new_connection().map_err(|e| format!("netlink: {e}"))?;
    tokio::spawn(connection);
    Ok(handle)
}

/// List interfaces via RTM_GETLINK; returns error when netlink unavailable.
pub async fn list_interfaces_netlink() -> Result<Vec<NetworkInterface>, String> {
    let handle = netlink_handle().await?;

    let mut out = Vec::new();
    let mut links = handle.link().get().execute();
    while let Some(msg) = links
        .try_next()
        .await
        .map_err(|e| format!("netlink: {e}"))?
    {
        let Some(name) = ifname_from_msg(&msg) else {
            continue;
        };
        let sysfs = std::path::Path::new("/sys/class/net").join(&name);
        out.push(NetworkInterface {
            name: name.clone(),
            r#type: classify_iface(&name, &sysfs).to_string(),
            state: operstate_from_flags(&msg.header.flags),
        });
    }
    if out.is_empty() {
        return Err("netlink returned no interfaces".into());
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Bring interface up/down via RTM_SETLINK.
pub async fn set_interface_state_netlink(name: &str, up: bool) -> Result<(), String> {
    reject_loopback_mutation(name)?;

    let handle = netlink_handle().await?;
    let mut links = handle.link().get().execute();
    while let Some(msg) = links
        .try_next()
        .await
        .map_err(|e| format!("netlink: {e}"))?
    {
        if ifname_from_msg(&msg).as_deref() != Some(name) {
            continue;
        }
        let index = msg.header.index;
        let mut req = handle.link().set(index);
        if up {
            req = req.up();
        } else {
            req = req.down();
        }
        req.execute()
            .await
            .map_err(|e| format!("netlink setlink: {e}"))?;
        return Ok(());
    }
    Err(format!("unknown interface: {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn netlink_list_or_skip() {
        match list_interfaces_netlink().await {
            Ok(ifaces) => assert!(ifaces.iter().any(|i| i.name == "lo")),
            Err(e) => assert!(e.contains("netlink")),
        }
    }
}
