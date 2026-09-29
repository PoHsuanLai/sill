//! What the wifi service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::{AccessPoint, Radio, Ssid};
use crate::units::Presence;

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

/// What the Wi-Fi service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WifiCommand {
    /// `WirelessEnabled` on or off.
    Radio(Radio),
    /// `RequestScan` on the wireless device (the control center's list opening).
    Scan,
    /// Activate the saved connection for `ssid` on the wireless device. A network with no saved
    /// connection is dropped: joining a new network needs a password prompt, which is a later
    /// milestone's (it opens COSMIC Settings' Wi-Fi page meanwhile).
    Connect(Ssid),
    /// Deactivate the wireless device's active connection.
    Disconnect,
}
