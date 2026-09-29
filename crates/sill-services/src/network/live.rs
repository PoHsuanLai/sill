//! NetworkManager's primary connection over the system bus: seeds a real first value, then
//! feeds every property change through [`reduce_network`].

use std::pin::Pin;

use cosmic_dbus_networkmanager::interface::NetworkManagerProxy;
use cosmic_dbus_networkmanager::interface::access_point::AccessPointProxy;
use cosmic_dbus_networkmanager::interface::active_connection::ActiveConnectionProxy;
use cosmic_dbus_networkmanager::interface::device::wireless::WirelessDeviceProxy;
use tokio::sync::{mpsc, watch};
use zbus::Connection;
use zbus::export::futures_core::Stream;
use zbus::zvariant::OwnedObjectPath;

use super::NetworkCommand;
use super::model::{Activating, Connectivity, Link, Network, NetworkEvent, reduce_network};
use crate::units::Percent;

/// Connects to NetworkManager and runs until `commands` closes (the `Control` was dropped). If
/// the bus or NetworkManager is unreachable, `tx` is left at its seeded default (disconnected).
pub(super) async fn run(
    tx: watch::Sender<Network>,
    mut commands: mpsc::UnboundedReceiver<NetworkCommand>,
) {
    let Ok(connection) = zbus::Connection::system().await else {
        return;
    };
    let Ok(nm) = NetworkManagerProxy::new(&connection).await else {
        return;
    };

    refresh(&connection, &nm, &tx).await;

    let mut primary = nm.receive_primary_connection_changed().await;
    let mut activating = nm.receive_activating_connection_changed().await;
    let mut connectivity = nm.receive_connectivity_changed().await;

    loop {
        tokio::select! {
            Some(_) = next(&mut primary) => {
                apply(&tx, NetworkEvent::Link(primary_link(&connection, &nm).await));
            }
            Some(_) = next(&mut activating) => {
                apply(&tx, NetworkEvent::Activating(activating_status(&connection, &nm).await));
            }
            Some(change) = next(&mut connectivity) => {
                if let Ok(value) = change.get().await {
                    apply(&tx, NetworkEvent::Connectivity(Connectivity::from_nm(value)));
                }
            }
            command = commands.recv() => match command {
                Some(NetworkCommand::Refresh) => refresh(&connection, &nm, &tx).await,
                None => break,
            }
        }
    }
}

/// Reads the primary connection, `ActivatingConnection` and connectivity once and sends the
/// resulting `Network`.
async fn refresh(
    connection: &Connection,
    nm: &NetworkManagerProxy<'_>,
    tx: &watch::Sender<Network>,
) {
    apply(tx, NetworkEvent::Link(primary_link(connection, nm).await));
    apply(
        tx,
        NetworkEvent::Activating(activating_status(connection, nm).await),
    );
    if let Ok(value) = nm.connectivity().await {
        apply(tx, NetworkEvent::Connectivity(Connectivity::from_nm(value)));
    }
}

/// Reduces `event` onto the receiver's current value and publishes the result.
fn apply(tx: &watch::Sender<Network>, event: NetworkEvent) {
    let next = reduce_network(tx.borrow().clone(), event);
    let _ = tx.send(next);
}

/// The primary connection's link, or `Disconnected` if there is none or it cannot be read.
async fn primary_link(connection: &Connection, nm: &NetworkManagerProxy<'_>) -> Link {
    let Ok(path) = nm.primary_connection().await else {
        return Link::Disconnected;
    };
    if path.as_str() == "/" {
        return Link::Disconnected;
    }
    let Ok(builder) = ActiveConnectionProxy::builder(connection).path(path) else {
        return Link::Disconnected;
    };
    let Ok(active) = builder.build().await else {
        return Link::Disconnected;
    };

    let state = active.state().await.unwrap_or(0);
    let kind = active.type_().await.unwrap_or_default();
    match state {
        1 => Link::Connecting,
        2 => link_of(connection, &active, &kind).await,
        _ => Link::Disconnected,
    }
}

/// `ActivatingConnection`'s status for the reducer: `Joining` for a non-`/` connection whose
/// `State` is 1 (activating) and whose `Type` is `802-11-wireless`, else `Idle` — for no
/// connection activating, one of another type, or one that cannot be read. The reducer, not this
/// read, decides whether an `Idle` report ends a running join as a failure or holds.
async fn activating_status(connection: &Connection, nm: &NetworkManagerProxy<'_>) -> Activating {
    let Ok(path) = nm.activating_connection().await else {
        return Activating::Idle;
    };
    if path.as_str() == "/" {
        return Activating::Idle;
    }
    let Ok(builder) = ActiveConnectionProxy::builder(connection).path(path) else {
        return Activating::Idle;
    };
    let Ok(active) = builder.build().await else {
        return Activating::Idle;
    };
    let state = active.state().await.unwrap_or(0);
    let kind = active.type_().await.unwrap_or_default();
    match (state, kind.as_str()) {
        (1, "802-11-wireless") => Activating::Joining,
        _ => Activating::Idle,
    }
}

/// The connected link for an active connection whose D-Bus `Type` is `kind`.
async fn link_of(connection: &Connection, active: &ActiveConnectionProxy<'_>, kind: &str) -> Link {
    match kind {
        "802-11-wireless" => wifi_details(connection, active).await,
        "802-3-ethernet" => Link::Wired,
        _ => Link::Other,
    }
}

/// A Wi-Fi active connection's SSID and signal strength, read from its device's active access
/// point; `Other` if neither device offers one (a race with disassociation, or a driver that
/// does not expose it).
async fn wifi_details(connection: &Connection, active: &ActiveConnectionProxy<'_>) -> Link {
    let Ok(devices) = active.devices().await else {
        return Link::Other;
    };
    for device in devices {
        if let Some(link) = access_point_link(connection, &device).await {
            return link;
        }
    }
    Link::Other
}

/// The Wi-Fi link for `device`'s active access point, or `None` if `device` is not a wireless
/// device or has no active access point.
async fn access_point_link(connection: &Connection, device: &OwnedObjectPath) -> Option<Link> {
    let wireless = WirelessDeviceProxy::builder(connection)
        .path(device.clone())
        .ok()?
        .build()
        .await
        .ok()?;
    let ap_path = wireless.active_access_point().await.ok()?;
    if ap_path.as_str() == "/" {
        return None;
    }
    let ap = AccessPointProxy::builder(connection)
        .path(ap_path)
        .ok()?
        .build()
        .await
        .ok()?;
    let ssid = ap.ssid().await.unwrap_or_default();
    let strength = ap.strength().await.unwrap_or(0);
    Some(Link::Wifi {
        ssid: String::from_utf8_lossy(&ssid).into_owned(),
        strength: Percent(u16::from(strength)),
    })
}

/// Polls one `Unpin` stream for its next item, for use as a `tokio::select!` branch.
async fn next<S>(stream: &mut S) -> Option<S::Item>
where
    S: Stream + Unpin,
{
    std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}
