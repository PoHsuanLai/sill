//! The control center's words: each tile's status line and each list row's detail, pure over
//! the service values.

use sill_model::bluetooth::{Bluetooth, BtBattery, BtConnection, BtDevice, BtPairing, BtPower};
use sill_model::network::{Link, Network};
use sill_model::power_profile::Profile;
use sill_model::units::Presence;
use sill_model::wifi::{AccessPoint, Association, Radio, Saved, Wifi};
use sill_settings::FocusMode;

/// The Wi-Fi tile's status line: "Off", "Unavailable" (no device, or blocked in hardware),
/// the connected network's name, "Connecting…", or "Not connected".
pub fn wifi_status(wifi: &Wifi, network: &Network) -> String {
    match (wifi.device, wifi.radio, &network.link) {
        (Presence::Absent, _, _) | (_, Radio::Blocked, _) => "Unavailable".to_owned(),
        (_, Radio::Off, _) => "Off".to_owned(),
        (_, Radio::On, Link::Wifi { ssid, .. }) => ssid.clone(),
        (_, Radio::On, Link::Connecting) => "Connecting…".to_owned(),
        (_, Radio::On, Link::Disconnected | Link::Wired | Link::Other) => {
            "Not connected".to_owned()
        }
    }
}

/// The Bluetooth tile's status line: "Unavailable" (no adapter), "Off" or "On". No device name:
/// several can be connected at once, and the device list is one chevron away.
pub fn bluetooth_tile_status(bluetooth: &Bluetooth) -> &'static str {
    match (bluetooth.presence, bluetooth.power) {
        (Presence::Absent, _) => "Unavailable",
        (Presence::Present, BtPower::Off) => "Off",
        (Presence::Present, BtPower::On) => "On",
    }
}

/// The bar's Bluetooth item's tooltip line: "Unavailable" (no adapter), "Off", the connected
/// devices' names joined with ", " (at most two, then "and N more"), or "On".
pub fn bluetooth_status(bluetooth: &Bluetooth) -> String {
    let connected: Vec<&str> = bluetooth
        .devices
        .iter()
        .filter(|d| d.connection == BtConnection::Connected)
        .map(|d| d.name.as_str())
        .collect();
    match (bluetooth.presence, bluetooth.power, connected.as_slice()) {
        (Presence::Absent, _, _) => "Unavailable".to_owned(),
        (_, BtPower::Off, _) => "Off".to_owned(),
        (_, BtPower::On, []) => "On".to_owned(),
        (_, BtPower::On, [one]) => (*one).to_owned(),
        (_, BtPower::On, [one, two]) => format!("{one}, {two}"),
        (_, BtPower::On, [one, two, rest @ ..]) => {
            format!("{one}, {two} and {} more", rest.len())
        }
    }
}

/// The Focus tile's status line: "On" or "Off".
pub fn focus_status(mode: FocusMode) -> &'static str {
    match mode {
        FocusMode::DoNotDisturb => "On",
        FocusMode::Off => "Off",
    }
}

/// A power profile's segment label: "Saver", "Balanced", "Performance".
pub fn profile_label(profile: Profile) -> &'static str {
    match profile {
        Profile::PowerSaver => "Saver",
        Profile::Balanced => "Balanced",
        Profile::Performance => "Performance",
    }
}

/// A network row's detail: "Connected", "Connecting…", "Saved", or "" for a new one; a secured
/// network the row also marks with a lock glyph, not words.
pub fn network_detail(access_point: &AccessPoint, saved: Saved) -> String {
    match (access_point.association, saved) {
        (Association::Connected, _) => "Connected",
        (Association::Connecting, _) => "Connecting…",
        (Association::Idle, Saved::Known) => "Saved",
        (Association::Idle, Saved::New) => "",
    }
    .to_owned()
}

/// A device row's detail: "Connected", "Connecting…", "Not connected" (paired) or "" (not
/// paired), then " · 80%" when a device that is not connected reports a battery (a connected
/// one's battery is the row's trailing ring, design/26 G21).
pub fn device_detail(device: &BtDevice) -> String {
    let state = match (device.connection, device.pairing) {
        (BtConnection::Connected, _) => "Connected",
        (BtConnection::Connecting, _) => "Connecting…",
        (BtConnection::Disconnected, BtPairing::Paired) => "Not connected",
        (BtConnection::Disconnected, BtPairing::Unpaired) => "",
    };
    match (device.connection, state, device.battery) {
        (BtConnection::Connected, state, _) | (_, state, BtBattery::Unknown) => state.to_owned(),
        (_, "", BtBattery::Level(level)) => format!("{}%", level.0),
        (_, state, BtBattery::Level(level)) => format!("{state} · {}%", level.0),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bluetooth_status, bluetooth_tile_status, device_detail, focus_status, network_detail,
        profile_label, wifi_status,
    };
    use sill_model::bluetooth::{
        Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing, BtPower,
    };
    use sill_model::network::{Link, Network};

    use sill_model::power_profile::Profile;
    use sill_model::units::{Percent, Presence};
    use sill_model::wifi::{AccessPoint, Association, Radio, Saved, Security, Ssid, Wifi};
    use sill_settings::FocusMode;

    fn wifi(radio: Radio) -> Wifi {
        Wifi {
            device: Presence::Present,
            radio,
            ..Wifi::default()
        }
    }

    fn device(name: &str, connection: BtConnection, battery: BtBattery) -> BtDevice {
        BtDevice {
            address: BtAddress(name.to_owned()),
            name: name.to_owned(),
            kind: BtKind::Headphones,
            connection,
            pairing: BtPairing::Paired,
            battery,
        }
    }

    #[test]
    fn tile_status_lines() {
        let home = Network {
            link: Link::Wifi {
                ssid: "home".to_owned(),
                strength: Percent(70),
            },
            ..Network::default()
        };
        assert_eq!(wifi_status(&wifi(Radio::On), &home), "home");
        assert_eq!(wifi_status(&wifi(Radio::Off), &home), "Off");
        assert_eq!(wifi_status(&wifi(Radio::Blocked), &home), "Unavailable");
        assert_eq!(wifi_status(&Wifi::default(), &home), "Unavailable");
        assert_eq!(
            wifi_status(&wifi(Radio::On), &Network::default()),
            "Not connected"
        );

        let on = |devices| Bluetooth {
            presence: Presence::Present,
            power: BtPower::On,
            devices,
            connect: Default::default(),
        };
        let connected = |name| device(name, BtConnection::Connected, BtBattery::Unknown);
        assert_eq!(bluetooth_status(&Bluetooth::default()), "Unavailable");
        assert_eq!(bluetooth_status(&on(Vec::new())), "On");
        assert_eq!(bluetooth_status(&on(vec![connected("Buds")])), "Buds");
        assert_eq!(
            bluetooth_status(&on(vec![connected("A"), connected("B"), connected("C")])),
            "A, B and 1 more"
        );
        let off = Bluetooth {
            power: BtPower::Off,
            ..on(Vec::new())
        };
        assert_eq!(bluetooth_status(&off), "Off");
        // The tile names no device, however many are connected.
        let two = on(vec![connected("Keyboard"), connected("Buds")]);
        assert_eq!(bluetooth_tile_status(&two), "On");
        assert_eq!(bluetooth_tile_status(&on(vec![connected("Buds")])), "On");
        assert_eq!(bluetooth_tile_status(&off), "Off");
        assert_eq!(bluetooth_tile_status(&Bluetooth::default()), "Unavailable");

        assert_eq!(focus_status(FocusMode::DoNotDisturb), "On");
        assert_eq!(focus_status(FocusMode::Off), "Off");
        assert_eq!(profile_label(Profile::PowerSaver), "Saver");
        assert_eq!(profile_label(Profile::Balanced), "Balanced");
        assert_eq!(profile_label(Profile::Performance), "Performance");
    }

    #[test]
    fn row_details() {
        let ap = |association| AccessPoint {
            ssid: Ssid("home".to_owned()),
            strength: Percent(60),
            security: Security::Secured,
            association,
        };
        assert_eq!(
            network_detail(&ap(Association::Connected), Saved::Known),
            "Connected"
        );
        assert_eq!(
            network_detail(&ap(Association::Connecting), Saved::Known),
            "Connecting…"
        );
        assert_eq!(
            network_detail(&ap(Association::Idle), Saved::Known),
            "Saved"
        );
        assert_eq!(network_detail(&ap(Association::Idle), Saved::New), "");

        let level = BtBattery::Level(Percent(80));
        assert_eq!(
            device_detail(&device("Buds", BtConnection::Connected, level)),
            "Connected"
        );
        assert_eq!(
            device_detail(&device("Buds", BtConnection::Connecting, level)),
            "Connecting… · 80%"
        );
        assert_eq!(
            device_detail(&device(
                "Buds",
                BtConnection::Disconnected,
                BtBattery::Unknown
            )),
            "Not connected"
        );
        let stranger = BtDevice {
            pairing: BtPairing::Unpaired,
            ..device("Tv", BtConnection::Disconnected, BtBattery::Unknown)
        };
        assert_eq!(device_detail(&stranger), "");
    }
}
