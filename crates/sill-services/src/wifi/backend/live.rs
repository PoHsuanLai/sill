//! NetworkManager's wireless side over the system bus: `WirelessEnabled` and
//! `WirelessHardwareEnabled`, the wireless device's access points and `LastScan`, the saved
//! connections' SSIDs, and the active access point; every change through
//! [`sill_model::wifi::step`]; [`WifiCommand`]s carried out.
//!
//! Same bus and proxies as the network service (`crate::nm`). NetworkManager starting after sill
//! or restarting under it is followed through its bus name ([`crate::owner`]), and the wireless
//! device is looked for again whenever one is added or removed (a USB adapter plugged in later).
//! Access point signals arrive in floods during a scan, so they are folded into one refresh a
//! short while after the last; every call to NetworkManager has a deadline.

use std::collections::HashMap;
use std::time::Duration;

use sill_model::units::Presence;
use sill_model::wifi::{Radio, Wifi, WifiCommand, WifiEvent};
use tokio::sync::{mpsc, watch};
use tokio::time::Instant;
use zbus::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};

use super::reads::{apply, refresh_access_points, refresh_radio, refresh_saved, saved_ssid_of};
use crate::deadline::{BUS_CALL, NETWORK_REFRESH, call, within};
use crate::env::Bus;
use crate::error::ServiceError;
use crate::nm::{DeviceProxy, DeviceType, NetworkManagerProxy, SettingsProxy, WirelessDeviceProxy};
use crate::owner::{self, Ended};
use crate::stream::{next, sleep_until_opt};

const SERVICE: &str = "org.freedesktop.NetworkManager";

/// How long after the last access point signal the list is read again: a scan reports every
/// point it finds, one signal each, and one read after the burst shows the same list.
const REFRESH_AFTER_SIGNALS: Duration = Duration::from_millis(250);

/// Runs until `commands` closes. Without NetworkManager or a wireless device `tx` stays at its
/// default, and it comes back by itself when either appears.
pub(crate) async fn run(
    bus: Bus,
    tx: watch::Sender<Wifi>,
    mut commands: mpsc::UnboundedReceiver<WifiCommand>,
) -> Result<(), ServiceError> {
    let connection = bus.connect_waiting().await?;
    owner::follow_owner(
        &connection,
        SERVICE,
        &mut commands,
        async |commands| session(&connection, &tx, commands).await,
        |_| apply(&tx, WifiEvent::Gone),
    )
    .await;
    Ok(())
}

/// How following one wireless device ended.
enum Followed {
    /// The commands closed: the service is done.
    Commands,
    /// The device left, or another took its place.
    DeviceGone,
}

/// One NetworkManager lifetime: find a wireless device, follow it, find one again when it goes.
async fn session(
    connection: &Connection,
    tx: &watch::Sender<Wifi>,
    commands: &mut mpsc::UnboundedReceiver<WifiCommand>,
) -> Ended {
    let (Ok(nm), Ok(settings)) = (
        NetworkManagerProxy::new(connection).await,
        SettingsProxy::new(connection).await,
    ) else {
        return Ended::Unreadable;
    };
    let (Ok(mut added), Ok(mut removed)) = (
        nm.receive_device_added().await,
        nm.receive_device_removed().await,
    ) else {
        return Ended::Unreadable;
    };
    loop {
        let found = within(
            NETWORK_REFRESH,
            "NetworkManager",
            find_wireless_device(connection, &nm),
        )
        .await
        .ok()
        .flatten();
        let Some(device_path) = found else {
            tokio::select! {
                Some(_) = next(&mut added) => {}
                command = commands.recv() => {
                    if command.is_none() {
                        return Ended::Commands;
                    }
                }
            }
            continue;
        };
        let followed = follow(
            connection,
            &nm,
            &settings,
            &device_path,
            tx,
            commands,
            &mut removed,
        )
        .await;
        match followed {
            Followed::Commands => return Ended::Commands,
            Followed::DeviceGone => {
                apply(tx, WifiEvent::Device(Presence::Absent));
                apply(tx, WifiEvent::AccessPoints(Vec::new()));
            }
        }
    }
}

/// Follow the wireless device at `device_path` until it goes or the commands close.
async fn follow(
    connection: &Connection,
    nm: &NetworkManagerProxy<'_>,
    settings: &SettingsProxy<'_>,
    device_path: &OwnedObjectPath,
    tx: &watch::Sender<Wifi>,
    commands: &mut mpsc::UnboundedReceiver<WifiCommand>,
    removed: &mut (impl zbus::export::futures_core::Stream + Unpin),
) -> Followed {
    let (Some(wireless), Some(device)) = (
        wireless_proxy(connection, device_path).await,
        device_proxy(connection, device_path).await,
    ) else {
        return Followed::DeviceGone;
    };
    let (Ok(mut ap_added), Ok(mut ap_removed), Ok(mut new_connection), Ok(mut connection_removed)) = (
        wireless.receive_access_point_added().await,
        wireless.receive_access_point_removed().await,
        settings.receive_new_connection().await,
        settings.receive_connection_removed().await,
    ) else {
        return Followed::DeviceGone;
    };

    apply(tx, WifiEvent::Device(Presence::Present));
    refresh_radio(nm, tx).await;
    refresh_access_points(connection, &wireless, &device, tx).await;
    refresh_saved(connection, settings, tx).await;

    let mut wireless_enabled = nm.receive_wireless_enabled_changed().await;
    let mut wireless_hw_enabled = nm.receive_wireless_hardware_enabled_changed().await;
    let mut last_scan = wireless.receive_last_scan_changed().await;
    let mut active_ap = wireless.receive_active_access_point_changed().await;
    // When the access point list is next read, once the signals about it have died down.
    let mut refresh_at: Option<Instant> = None;

    loop {
        tokio::select! {
            Some(_) = next(&mut wireless_enabled) => refresh_radio(nm, tx).await,
            Some(_) = next(&mut wireless_hw_enabled) => refresh_radio(nm, tx).await,
            Some(_) = next(&mut ap_added) => refresh_at = Some(Instant::now() + REFRESH_AFTER_SIGNALS),
            Some(_) = next(&mut ap_removed) => refresh_at = Some(Instant::now() + REFRESH_AFTER_SIGNALS),
            Some(_) = next(&mut last_scan) => refresh_at = Some(Instant::now() + REFRESH_AFTER_SIGNALS),
            Some(_) = next(&mut active_ap) => refresh_at = Some(Instant::now() + REFRESH_AFTER_SIGNALS),
            () = sleep_until_opt(refresh_at) => {
                refresh_at = None;
                refresh_access_points(connection, &wireless, &device, tx).await;
            }
            Some(_) = next(&mut new_connection) => refresh_saved(connection, settings, tx).await,
            Some(_) = next(&mut connection_removed) => refresh_saved(connection, settings, tx).await,
            Some(_) = next(removed) => {
                let still = within(NETWORK_REFRESH, "NetworkManager", find_wireless_device(connection, nm))
                    .await
                    .ok()
                    .flatten();
                if still.as_ref() != Some(device_path) {
                    return Followed::DeviceGone;
                }
            }
            command = commands.recv() => match command {
                Some(command) => carry_out(connection, nm, settings, &wireless, &device, device_path, command).await,
                None => return Followed::Commands,
            }
        }
    }
}

/// Carry out one command; every call to NetworkManager has [`BUS_CALL`] to answer, and a refusal
/// or silence leaves things as they were.
async fn carry_out(
    connection: &Connection,
    nm: &NetworkManagerProxy<'_>,
    settings: &SettingsProxy<'_>,
    wireless: &WirelessDeviceProxy<'_>,
    device: &DeviceProxy<'_>,
    device_path: &OwnedObjectPath,
    command: WifiCommand,
) {
    let _ = match command {
        WifiCommand::Radio(radio) => {
            call(
                BUS_CALL,
                "NetworkManager",
                nm.set_wireless_enabled(radio == Radio::On),
            )
            .await
        }
        WifiCommand::Scan => {
            call(
                BUS_CALL,
                "NetworkManager",
                wireless.request_scan(HashMap::new()),
            )
            .await
        }
        WifiCommand::Connect(ssid) => {
            within(
                NETWORK_REFRESH,
                "NetworkManager",
                connect(connection, nm, settings, device_path, &ssid),
            )
            .await
        }
        WifiCommand::Disconnect => call(BUS_CALL, "NetworkManager", device.disconnect()).await,
    };
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

/// Activates the saved connection for `ssid`, if one is found; a network with none is dropped
/// ([`WifiCommand::Connect`]'s own rule).
async fn connect(
    connection: &Connection,
    nm: &NetworkManagerProxy<'_>,
    settings: &SettingsProxy<'_>,
    device_path: &OwnedObjectPath,
    ssid: &sill_model::wifi::Ssid,
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
