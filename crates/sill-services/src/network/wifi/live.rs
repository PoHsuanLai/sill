//! NetworkManager's wireless side over the system bus: `WirelessEnabled` and
//! `WirelessHardwareEnabled`, the wireless device's access points and `LastScan`, the saved
//! connections' SSIDs, and the active access point; every change through
//! [`super::reduce_wifi`]; [`WifiCommand`]s carried out.
//!
//! Same crate and bus as [`crate::network::live`] (`cosmic_dbus_networkmanager`); the wireless
//! device is found once by its `DeviceType` and kept for the life of the task (a device that
//! disappears mid-run is not re-found — `Refresh` is a later milestone's, per [`WifiCommand`]).

use std::collections::HashMap;
use std::pin::Pin;

use cosmic_dbus_networkmanager::interface::NetworkManagerProxy;
use cosmic_dbus_networkmanager::interface::access_point::AccessPointProxy;
use cosmic_dbus_networkmanager::interface::device::DeviceProxy;
use cosmic_dbus_networkmanager::interface::device::wireless::WirelessDeviceProxy;
use cosmic_dbus_networkmanager::interface::enums::{DeviceState, DeviceType};
use cosmic_dbus_networkmanager::interface::settings::SettingsProxy;
use cosmic_dbus_networkmanager::interface::settings::connection::ConnectionSettingsProxy;
use tokio::sync::{mpsc, watch};
use zbus::Connection;
use zbus::export::futures_core::Stream;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};

use super::WifiCommand;
use super::model::{AccessPoint, Association, Radio, Security, Ssid, Wifi, WifiEvent, reduce_wifi};
use crate::units::{Percent, Presence};

/// Runs until `commands` closes. Without a bus, NetworkManager or a wireless device, `tx` stays
/// at its default.
pub(super) async fn run(
    tx: watch::Sender<Wifi>,
    mut commands: mpsc::UnboundedReceiver<WifiCommand>,
) {
    let Ok(connection) = Connection::system().await else {
        return;
    };
    let Ok(nm) = NetworkManagerProxy::new(&connection).await else {
        return;
    };
    let Ok(settings) = SettingsProxy::new(&connection).await else {
        return;
    };
    let Some(device_path) = find_wireless_device(&connection, &nm).await else {
        return;
    };
    let Some(wireless) = wireless_proxy(&connection, &device_path).await else {
        return;
    };
    let Some(device) = device_proxy(&connection, &device_path).await else {
        return;
    };
    let Ok(mut ap_added) = wireless.receive_access_point_added().await else {
        return;
    };
    let Ok(mut ap_removed) = wireless.receive_access_point_removed().await else {
        return;
    };
    let Ok(mut new_connection) = settings.receive_new_connection().await else {
        return;
    };
    let Ok(mut connection_removed) = settings.receive_connection_removed().await else {
        return;
    };

    apply(&tx, WifiEvent::Device(Presence::Present));
    refresh_radio(&nm, &tx).await;
    refresh_access_points(&connection, &wireless, &device, &tx).await;
    refresh_saved(&connection, &settings, &tx).await;

    let mut wireless_enabled = nm.receive_wireless_enabled_changed().await;
    let mut wireless_hw_enabled = nm.receive_wireless_hardware_enabled_changed().await;
    let mut last_scan = wireless.receive_last_scan_changed().await;
    let mut active_ap = wireless.receive_active_access_point_changed().await;

    loop {
        tokio::select! {
            Some(_) = next(&mut wireless_enabled) => refresh_radio(&nm, &tx).await,
            Some(_) = next(&mut wireless_hw_enabled) => refresh_radio(&nm, &tx).await,
            Some(_) = next(&mut ap_added) => refresh_access_points(&connection, &wireless, &device, &tx).await,
            Some(_) = next(&mut ap_removed) => refresh_access_points(&connection, &wireless, &device, &tx).await,
            Some(_) = next(&mut last_scan) => refresh_access_points(&connection, &wireless, &device, &tx).await,
            Some(_) = next(&mut active_ap) => refresh_access_points(&connection, &wireless, &device, &tx).await,
            Some(_) = next(&mut new_connection) => refresh_saved(&connection, &settings, &tx).await,
            Some(_) = next(&mut connection_removed) => refresh_saved(&connection, &settings, &tx).await,
            command = commands.recv() => match command {
                Some(WifiCommand::Radio(radio)) => {
                    let _ = nm.set_wireless_enabled(radio == Radio::On).await;
                }
                Some(WifiCommand::Scan) => {
                    let _ = wireless.request_scan(HashMap::new()).await;
                }
                Some(WifiCommand::Connect(ssid)) => {
                    connect(&connection, &nm, &settings, &device_path, &ssid).await;
                }
                Some(WifiCommand::Disconnect) => {
                    let _ = device.disconnect().await;
                }
                None => break,
            }
        }
    }
}

/// The path of NetworkManager's first Wi-Fi device, or `None` without one.
async fn find_wireless_device(
    connection: &Connection,
    nm: &NetworkManagerProxy<'_>,
) -> Option<OwnedObjectPath> {
    let devices = nm.get_all_devices().await.ok()?;
    for path in devices {
        let device = device_proxy(connection, &path).await?;
        if device.device_type().await.map(DeviceType::from).ok() == Some(DeviceType::Wifi) {
            return Some(path);
        }
    }
    None
}

async fn wireless_proxy<'a>(
    connection: &'a Connection,
    path: &OwnedObjectPath,
) -> Option<WirelessDeviceProxy<'a>> {
    WirelessDeviceProxy::builder(connection)
        .path(path.clone())
        .ok()?
        .build()
        .await
        .ok()
}

async fn device_proxy<'a>(
    connection: &'a Connection,
    path: &OwnedObjectPath,
) -> Option<DeviceProxy<'a>> {
    DeviceProxy::builder(connection)
        .path(path.clone())
        .ok()?
        .build()
        .await
        .ok()
}

/// Reads `WirelessEnabled` and `WirelessHardwareEnabled` and publishes the radio they describe.
async fn refresh_radio(nm: &NetworkManagerProxy<'_>, tx: &watch::Sender<Wifi>) {
    let enabled = nm.wireless_enabled().await.unwrap_or(false);
    let hw_enabled = nm.wireless_hardware_enabled().await.unwrap_or(true);
    apply(tx, WifiEvent::Radio(radio_of(enabled, hw_enabled)));
}

fn radio_of(enabled: bool, hw_enabled: bool) -> Radio {
    if !hw_enabled {
        Radio::Blocked
    } else if enabled {
        Radio::On
    } else {
        Radio::Off
    }
}

/// Reads every access point the device currently sees and publishes the list.
async fn refresh_access_points(
    connection: &Connection,
    wireless: &WirelessDeviceProxy<'_>,
    device: &DeviceProxy<'_>,
    tx: &watch::Sender<Wifi>,
) {
    let Ok(paths) = wireless.access_points().await else {
        return;
    };
    let active_path = wireless.active_access_point().await.ok();
    let state = device
        .state()
        .await
        .map(DeviceState::from)
        .unwrap_or_default();
    let mut points = Vec::with_capacity(paths.len());
    for path in paths {
        if let Some(ap) = access_point_of(connection, &path, active_path.as_ref(), state).await {
            points.push(ap);
        }
    }
    apply(tx, WifiEvent::AccessPoints(points));
}

async fn access_point_of(
    connection: &Connection,
    path: &OwnedObjectPath,
    active_path: Option<&OwnedObjectPath>,
    state: DeviceState,
) -> Option<AccessPoint> {
    let ap = AccessPointProxy::builder(connection)
        .path(path.clone())
        .ok()?
        .build()
        .await
        .ok()?;
    let ssid = ap.ssid().await.unwrap_or_default();
    let strength = ap.strength().await.unwrap_or(0);
    let flags = ap.flags().await.unwrap_or(0);
    let wpa_flags = ap.wpa_flags().await.unwrap_or(0);
    let rsn_flags = ap.rsn_flags().await.unwrap_or(0);
    let association = if active_path == Some(path) {
        association_of(state)
    } else {
        Association::Idle
    };
    Some(AccessPoint {
        ssid: Ssid(String::from_utf8_lossy(&ssid).into_owned()),
        strength: Percent(u16::from(strength)),
        security: security_of(flags, wpa_flags, rsn_flags),
        association,
    })
}

/// `Flags`' `NM_802_11_AP_FLAGS_PRIVACY` bit, or any `WpaFlags`/`RsnFlags` bit: a network with
/// none of these needs no secret to join.
fn security_of(flags: u32, wpa_flags: u32, rsn_flags: u32) -> Security {
    const PRIVACY: u32 = 0x1;
    if flags & PRIVACY != 0 || wpa_flags != 0 || rsn_flags != 0 {
        Security::Secured
    } else {
        Security::Open
    }
}

/// The active access point's association, from the device's own state (an access point has no
/// state of its own).
fn association_of(state: DeviceState) -> Association {
    match state {
        DeviceState::Activated => Association::Connected,
        DeviceState::Prepare
        | DeviceState::Config
        | DeviceState::NeedAuth
        | DeviceState::IpConfig
        | DeviceState::IpCheck
        | DeviceState::Secondaries => Association::Connecting,
        _ => Association::Idle,
    }
}

/// Reads every saved connection's `802-11-wireless` SSID and publishes the list.
async fn refresh_saved(
    connection: &Connection,
    settings: &SettingsProxy<'_>,
    tx: &watch::Sender<Wifi>,
) {
    let Ok(paths) = settings.connections().await else {
        return;
    };
    let mut ssids = Vec::with_capacity(paths.len());
    for path in paths {
        if let Some(ssid) = saved_ssid_of(connection, &path).await {
            ssids.push(ssid);
        }
    }
    apply(tx, WifiEvent::Saved(ssids));
}

/// A saved connection's `802-11-wireless.ssid`, when it has one (a wired or VPN connection does
/// not).
async fn saved_ssid_of(connection: &Connection, path: &OwnedObjectPath) -> Option<Ssid> {
    let settings = ConnectionSettingsProxy::builder(connection)
        .path(path.clone())
        .ok()?
        .build()
        .await
        .ok()?;
    let all = settings.get_settings().await.ok()?;
    let wireless = all.get("802-11-wireless")?;
    let ssid = wireless.get("ssid")?;
    let bytes = Vec::<u8>::try_from(ssid.clone()).ok()?;
    Some(Ssid(String::from_utf8_lossy(&bytes).into_owned()))
}

/// Activates the saved connection for `ssid`, if one is found; a network with none is dropped
/// ([`WifiCommand::Connect`]'s own rule).
async fn connect(
    connection: &Connection,
    nm: &NetworkManagerProxy<'_>,
    settings: &SettingsProxy<'_>,
    device_path: &OwnedObjectPath,
    ssid: &Ssid,
) {
    let Ok(paths) = settings.connections().await else {
        return;
    };
    for path in paths {
        if saved_ssid_of(connection, &path).await.as_ref() != Some(ssid) {
            continue;
        }
        let conn_path = ObjectPath::from(&path);
        let dev_path = ObjectPath::from(device_path);
        let _ = nm
            .activate_connection(&conn_path, &dev_path, &ObjectPath::default())
            .await;
        return;
    }
}

/// Reduces `event` onto the receiver's current value and publishes the result.
fn apply(tx: &watch::Sender<Wifi>, event: WifiEvent) {
    let next = reduce_wifi(tx.borrow().clone(), event);
    let _ = tx.send(next);
}

/// Polls one `Unpin` stream for its next item, for use as a `tokio::select!` branch.
async fn next<S>(stream: &mut S) -> Option<S::Item>
where
    S: Stream + Unpin,
{
    std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}
