//! Reading NetworkManager's wireless side: the radio, the access points, the saved connections.
//! Every read is bounded by [`NETWORK_REFRESH`], so NetworkManager hanging mid-list costs one
//! skipped refresh, not the service.

use sill_model::units::Percent;
use sill_model::wifi::{AccessPoint, Association, Radio, Security, Ssid, Wifi, WifiEvent};
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use crate::deadline::{NETWORK_REFRESH, within};
use crate::machine::Published;
use crate::nm::{
    AccessPointProxy, ConnectionSettingsProxy, DeviceProxy, DeviceState, NetworkManagerProxy,
    SettingsProxy, WirelessDeviceProxy,
};

/// The most access points listed. A crowded building shows a few hundred; the list in the
/// control center is useful for the strongest few dozen, and every entry costs bus calls.
pub(super) const MAX_ACCESS_POINTS: usize = 128;

/// Reads `WirelessEnabled` and `WirelessHardwareEnabled` and publishes the radio they describe.
pub(super) async fn refresh_radio(nm: &NetworkManagerProxy<'_>, tx: &Published<Wifi, WifiEvent>) {
    let read = within(NETWORK_REFRESH, "NetworkManager", async {
        let enabled = nm.wireless_enabled().await.unwrap_or(false);
        let hw_enabled = nm.wireless_hardware_enabled().await.unwrap_or(true);
        radio_of(enabled, hw_enabled)
    })
    .await;
    if let Ok(radio) = read {
        tx.feed(WifiEvent::Radio(radio));
    }
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

/// Reads the access points the device currently sees (at most [`MAX_ACCESS_POINTS`]) and
/// publishes the list.
pub(super) async fn refresh_access_points(
    connection: &Connection,
    wireless: &WirelessDeviceProxy<'_>,
    device: &DeviceProxy<'_>,
    tx: &Published<Wifi, WifiEvent>,
) {
    let read = within(NETWORK_REFRESH, "NetworkManager", async {
        let paths = wireless.access_points().await.ok()?;
        let active_path = wireless.active_access_point().await.ok();
        let state = device
            .state()
            .await
            .map(DeviceState::from)
            .unwrap_or_default();
        let mut points = Vec::with_capacity(paths.len().min(MAX_ACCESS_POINTS));
        for path in paths.into_iter().take(MAX_ACCESS_POINTS) {
            if let Some(ap) = access_point_of(connection, &path, active_path.as_ref(), state).await
            {
                points.push(ap);
            }
        }
        Some(points)
    })
    .await;
    if let Ok(Some(points)) = read {
        tx.feed(WifiEvent::AccessPoints(points));
    }
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
pub(super) async fn refresh_saved(
    connection: &Connection,
    settings: &SettingsProxy<'_>,
    tx: &Published<Wifi, WifiEvent>,
) {
    if let Ok(Some(ssids)) = within(
        NETWORK_REFRESH,
        "NetworkManager",
        saved_ssids(connection, settings),
    )
    .await
    {
        tx.feed(WifiEvent::Saved(ssids));
    }
}

async fn saved_ssids(connection: &Connection, settings: &SettingsProxy<'_>) -> Option<Vec<Ssid>> {
    let paths = settings.connections().await.ok()?;
    let mut ssids = Vec::with_capacity(paths.len());
    for path in paths {
        if let Some(ssid) = saved_ssid_of(connection, &path).await {
            ssids.push(ssid);
        }
    }
    Some(ssids)
}

/// A saved connection's `802-11-wireless.ssid`, when it has one (a wired or VPN connection does
/// not).
pub(super) async fn saved_ssid_of(connection: &Connection, path: &OwnedObjectPath) -> Option<Ssid> {
    let settings = ConnectionSettingsProxy::builder(connection)
        .path(path.clone())
        .ok()?
        .build()
        .await
        .ok()?;
    let mut all = settings.get_settings().await.ok()?;
    let ssid = all.remove("802-11-wireless")?.remove("ssid")?;
    let bytes = Vec::<u8>::try_from(ssid).ok()?;
    Some(Ssid(String::from_utf8_lossy(&bytes).into_owned()))
}

#[cfg(test)]
mod tests {
    use sill_model::wifi::{Radio, Security};

    use super::{radio_of, security_of};

    #[test]
    fn the_radio_reads_hardware_first_then_software() {
        const CASES: &[(&str, bool, bool, Radio)] = &[
            ("killed in hardware", true, false, Radio::Blocked),
            ("killed in hardware and off", false, false, Radio::Blocked),
            ("on", true, true, Radio::On),
            ("off", false, true, Radio::Off),
        ];
        for (name, enabled, hardware, want) in CASES {
            assert_eq!(radio_of(*enabled, *hardware), *want, "{name}");
        }
    }

    #[test]
    fn a_network_is_secured_by_any_privacy_or_protection_flag() {
        const CASES: &[(&str, u32, u32, u32, Security)] = &[
            ("open", 0, 0, 0, Security::Open),
            ("privacy", 1, 0, 0, Security::Secured),
            ("wpa", 0, 0x100, 0, Security::Secured),
            ("rsn", 0, 0, 0x200, Security::Secured),
        ];
        for (name, flags, wpa, rsn, want) in CASES {
            assert_eq!(security_of(*flags, *wpa, *rsn), *want, "{name}");
        }
    }
}
