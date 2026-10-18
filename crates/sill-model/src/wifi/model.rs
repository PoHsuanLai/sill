//! Wi-Fi's state and the network list's order.

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
        rank(a_ap, *a_saved)
            .cmp(&rank(b_ap, *b_saved))
            .then(b_ap.strength.cmp(&a_ap.strength))
            .then(a_ap.ssid.cmp(&b_ap.ssid))
    });
    rows
}

/// A row's sort key: connected first, then saved, then the rest (strength and name break ties
/// outside this key).
fn rank(ap: &AccessPoint, saved: Saved) -> (u8, u8) {
    let connected = u8::from(ap.association != Association::Connected);
    let known = u8::from(saved != Saved::Known);
    (connected, known)
}
