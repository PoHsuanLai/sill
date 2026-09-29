//! Wi-Fi's state, the network list's order, and the one reducer.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::units::{Percent, Presence};

/// The wireless radio and what it can see.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Wifi {
    /// Whether NetworkManager has a wireless device at all.
    pub device: Presence,
    pub radio: Radio,
    /// One entry per access point NetworkManager reports (a network with several access points
    /// appears several times; [`listed_networks`] folds them).
    pub access_points: Vec<AccessPoint>,
    /// The SSIDs with a saved connection, which [`WifiCommand::Connect`] can join.
    ///
    /// [`WifiCommand::Connect`]: super::WifiCommand::Connect
    pub saved: Vec<Ssid>,
}

/// The radio switch: `WirelessEnabled`, and `WirelessHardwareEnabled` (a hardware kill switch
/// or airplane key, which software cannot undo).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Radio {
    #[default]
    Off,
    On,
    /// Off in hardware: the module's toggle is disabled.
    Blocked,
}

/// A network name. SSIDs are bytes; non-UTF-8 ones are shown lossily, and matched by these
/// lossy strings (a saved connection is found the same way).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ssid(pub String);

/// One access point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessPoint {
    pub ssid: Ssid,
    /// `Strength`.
    pub strength: Percent,
    /// From `Flags`, `WpaFlags` and `RsnFlags`.
    pub security: Security,
    /// Whether this is the device's active access point.
    pub association: Association,
}

/// Whether joining needs a secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Security {
    #[default]
    Open,
    Secured,
}

/// An access point's relation to the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Association {
    #[default]
    Idle,
    /// The device is activating a connection to it.
    Connecting,
    Connected,
}

/// Whether a listed network has a saved connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Saved {
    Known,
    New,
}

/// One change NetworkManager reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum WifiEvent {
    /// A wireless device appeared (or the first one went and another took its place).
    Device(Presence),
    Radio(Radio),
    /// The device's whole access point list, after a scan or an added/removed point.
    AccessPoints(Vec<AccessPoint>),
    /// The saved connections' SSIDs.
    Saved(Vec<Ssid>),
    /// NetworkManager left the bus.
    Gone,
}

/// `wifi` after `event`. Radio `Off` or `Blocked` empties the access points (NetworkManager
/// keeps stale ones for a while).
pub fn step(wifi: Wifi, event: WifiEvent) -> Wifi {
    match event {
        WifiEvent::Device(device) => Wifi { device, ..wifi },
        WifiEvent::Radio(radio) => {
            let access_points = match radio {
                Radio::On => wifi.access_points,
                Radio::Off | Radio::Blocked => Vec::new(),
            };
            Wifi {
                radio,
                access_points,
                ..wifi
            }
        }
        WifiEvent::AccessPoints(access_points) => Wifi {
            access_points,
            ..wifi
        },
        WifiEvent::Saved(saved) => Wifi { saved, ..wifi },
        WifiEvent::Gone => Wifi::default(),
    }
}

/// The networks the control center lists: one per SSID (its strongest access point, and
/// `Connected` or `Connecting` if any of its points is), hidden (empty) SSIDs left out, ordered
/// connected first, then saved networks, then the rest, by strength (strongest first) and then
/// name within each group; each with whether it is saved.
pub fn listed_networks(wifi: &Wifi) -> Vec<(AccessPoint, Saved)> {
    let mut groups: HashMap<&Ssid, (AccessPoint, bool)> = HashMap::new();
    for ap in &wifi.access_points {
        if ap.ssid.0.is_empty() {
            continue;
        }
        let active = matches!(
            ap.association,
            Association::Connected | Association::Connecting
        );
        groups
            .entry(&ap.ssid)
            .and_modify(|(rep, seen_active)| {
                if ap.strength > rep.strength {
                    *rep = ap.clone();
                }
                *seen_active = *seen_active || active;
            })
            .or_insert_with(|| (ap.clone(), active));
    }

    let mut rows: Vec<(AccessPoint, Saved)> = groups
        .into_values()
        .map(|(mut rep, active)| {
            if active && rep.association == Association::Idle {
                rep.association = Association::Connecting;
            }
            let saved = if wifi.saved.contains(&rep.ssid) {
                Saved::Known
            } else {
                Saved::New
            };
            (rep, saved)
        })
        .collect();

    rows.sort_by(|(a_ap, a_saved), (b_ap, b_saved)| {
        rank(a_ap, a_saved)
            .cmp(&rank(b_ap, b_saved))
            .then(b_ap.strength.cmp(&a_ap.strength))
            .then(a_ap.ssid.cmp(&b_ap.ssid))
    });
    rows
}

/// A row's sort key: connected first, then saved, then the rest (strength and name break ties
/// outside this key).
fn rank(ap: &AccessPoint, saved: &Saved) -> (u8, u8) {
    let connected = u8::from(ap.association != Association::Connected);
    let known = u8::from(*saved != Saved::Known);
    (connected, known)
}

#[cfg(test)]
mod tests {
    use super::{
        AccessPoint, Association, Radio, Saved, Security, Ssid, Wifi, WifiEvent, listed_networks,
        step,
    };
    use crate::units::{Percent, Presence};

    fn ap(ssid: &str, strength: u16, association: Association) -> AccessPoint {
        AccessPoint {
            ssid: Ssid(ssid.to_owned()),
            strength: Percent(strength),
            security: Security::Secured,
            association,
        }
    }

    fn on(access_points: Vec<AccessPoint>, saved: &[&str]) -> Wifi {
        Wifi {
            device: Presence::Present,
            radio: Radio::On,
            access_points,
            saved: saved.iter().map(|s| Ssid((*s).to_owned())).collect(),
        }
    }

    #[test]
    fn one_row_per_network_connected_then_saved_then_strongest() {
        use Association::{Connected, Idle};
        let wifi = on(
            vec![
                ap("cafe", 90, Idle),
                ap("home", 40, Idle),
                ap("home", 60, Connected),
                ap("office", 30, Idle),
                ap("", 99, Idle),
                ap("attic", 90, Idle),
            ],
            &["office"],
        );
        let rows: Vec<(String, u16, Association, Saved)> = listed_networks(&wifi)
            .into_iter()
            .map(|(ap, saved)| (ap.ssid.0, ap.strength.0, ap.association, saved))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("home".to_owned(), 60, Connected, Saved::New),
                ("office".to_owned(), 30, Idle, Saved::Known),
                ("attic".to_owned(), 90, Idle, Saved::New),
                ("cafe".to_owned(), 90, Idle, Saved::New),
            ]
        );
    }

    #[test]
    fn events_reduce_to_the_expected_wifi() {
        let seen = || vec![ap("home", 60, Association::Connected)];
        let cases: Vec<(&str, Wifi, WifiEvent, Wifi)> = vec![
            (
                "a scan lands",
                on(Vec::new(), &[]),
                WifiEvent::AccessPoints(seen()),
                on(seen(), &[]),
            ),
            (
                "the radio goes off and the list with it",
                on(seen(), &["home"]),
                WifiEvent::Radio(Radio::Off),
                Wifi {
                    radio: Radio::Off,
                    ..on(Vec::new(), &["home"])
                },
            ),
            (
                "saved networks",
                on(seen(), &[]),
                WifiEvent::Saved(vec![Ssid("home".to_owned())]),
                on(seen(), &["home"]),
            ),
            (
                "networkmanager leaves",
                on(seen(), &["home"]),
                WifiEvent::Gone,
                Wifi::default(),
            ),
        ];
        for (name, before, event, want) in cases {
            assert_eq!(step(before, event), want, "{name}");
        }
    }
}
