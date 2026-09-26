//! The control center's words: each tile's status line and each list row's detail, pure over
//! the service values.

use sill_services::{
    AccessPoint, Association, Bluetooth, BtBattery, BtConnection, BtDevice, BtPairing, BtPower,
    Link, MediaPlayer, Network, Presence, Profile, Radio, Saved, Wifi,
};
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

/// The Bluetooth tile's status line: "Unavailable" (no adapter), "Off", the connected devices'
/// names joined with ", " (at most two, then "and N more"), or "On".
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
/// paired), then " · 80%" when the device reports a battery.
pub fn device_detail(device: &BtDevice) -> String {
    let state = match (device.connection, device.pairing) {
        (BtConnection::Connected, _) => "Connected",
        (BtConnection::Connecting, _) => "Connecting…",
        (BtConnection::Disconnected, BtPairing::Paired) => "Not connected",
        (BtConnection::Disconnected, BtPairing::Unpaired) => "",
    };
    match (state, device.battery) {
        (state, BtBattery::Unknown) => state.to_owned(),
        ("", BtBattery::Level(level)) => format!("{}%", level.0),
        (state, BtBattery::Level(level)) => format!("{state} · {}%", level.0),
    }
}

/// The Now Playing module's two lines: the title (the player's identity when there is none),
/// and the artists joined with ", " then " — " and the album, leaving out whichever is empty.
pub fn track_lines(player: &MediaPlayer) -> (String, String) {
    let track = &player.track;
    let title = if track.title.is_empty() {
        player.identity.clone()
    } else {
        track.title.clone()
    };
    let artists = track.artists.join(", ");
    let by = [artists.as_str(), track.album.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" — ");
    (title, by)
}

#[cfg(test)]
mod tests {
    use super::{
        bluetooth_status, device_detail, focus_status, network_detail, profile_label, track_lines,
        wifi_status,
    };
    use sill_services::{
        Abilities, AccessPoint, Association, Bluetooth, BtAddress, BtBattery, BtConnection,
        BtDevice, BtKind, BtPairing, BtPower, Link, MediaPlayer, Network, Percent, Playback,
        PlayerId, Presence, Profile, Radio, Saved, Security, Ssid, Stamp, Timeline, Track, Wifi,
    };
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
            "Connected · 80%"
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

    #[test]
    fn now_playing_lines() {
        let player = |title: &str, artists: &[&str], album: &str| MediaPlayer {
            id: PlayerId("org.mpris.MediaPlayer2.spotify".to_owned()),
            identity: "Spotify".to_owned(),
            playback: Playback::Playing,
            changed: Stamp(0),
            track: Track {
                title: title.to_owned(),
                artists: artists.iter().map(|a| (*a).to_owned()).collect(),
                album: album.to_owned(),
                ..Track::default()
            },
            timeline: Timeline::Unknown,
            abilities: Abilities::default(),
        };
        let lines = |p: MediaPlayer| track_lines(&p);
        assert_eq!(
            lines(player("So What", &["Miles Davis"], "Kind of Blue")),
            (
                "So What".to_owned(),
                "Miles Davis — Kind of Blue".to_owned()
            )
        );
        assert_eq!(
            lines(player("Duet", &["A", "B"], "")),
            ("Duet".to_owned(), "A, B".to_owned())
        );
        assert_eq!(
            lines(player("", &[], "")),
            ("Spotify".to_owned(), String::new())
        );
    }
}
