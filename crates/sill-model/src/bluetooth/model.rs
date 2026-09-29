//! Bluetooth's state and the device list's order.

use serde::{Deserialize, Serialize};

use crate::attempt::Attempts;
use crate::units::{Percent, Presence};

/// The first BlueZ adapter and its devices, as the control center draws them.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Bluetooth {
    /// Whether BlueZ answered with an adapter at all.
    pub presence: Presence,
    /// `Adapter1.Powered`.
    pub power: BtPower,
    /// Every device BlueZ knows under the adapter, in the order BlueZ reported them.
    pub devices: Vec<BtDevice>,
    /// The connect attempts: one stamp per device entering `Connecting`, a new one per connect
    /// that falls back to `Disconnected` (the bar's Bluetooth glyph, quire design/26 R4, R6).
    #[serde(default)]
    pub connect: Attempts,
}

/// The adapter's radio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BtPower {
    #[default]
    Off,
    On,
}

/// A device's `Address` (`AA:BB:CC:DD:EE:FF`): its identity across reconnects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BtAddress(pub String);

/// One device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BtDevice {
    pub address: BtAddress,
    /// `Alias`: the user's name for it, which BlueZ falls back to the device's own `Name` and
    /// then its address for.
    pub name: String,
    /// From `Device1.Icon` ([`super::device_kind`]).
    pub kind: BtKind,
    pub connection: BtConnection,
    pub pairing: BtPairing,
    /// `Battery1.Percentage`, where the device reports one.
    pub battery: BtBattery,
}

/// What sort of device it is: which glyph its row draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BtKind {
    Headphones,
    Headset,
    Speaker,
    Keyboard,
    Mouse,
    Gamepad,
    Phone,
    Computer,
    #[default]
    Other,
}

/// `Device1.Connected`, and a connect under way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BtConnection {
    #[default]
    Disconnected,
    /// A `Connect` was sent and has not answered yet.
    Connecting,
    Connected,
}

/// `Device1.Paired`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BtPairing {
    #[default]
    Unpaired,
    Paired,
}

/// A device's battery, where it has `org.bluez.Battery1`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BtBattery {
    #[default]
    Unknown,
    Level(Percent),
}

/// The devices the control center lists, in order: connected first, then the other paired
/// devices, then the rest; by name within each group. Unpaired devices are listed only while
/// the radio is on.
pub fn listed_devices(bluetooth: &Bluetooth) -> Vec<&BtDevice> {
    let mut devices: Vec<&BtDevice> = bluetooth
        .devices
        .iter()
        .filter(|d| bluetooth.power == BtPower::On || d.pairing == BtPairing::Paired)
        .collect();
    devices.sort_by(|a, b| {
        group_of(a)
            .cmp(&group_of(b))
            .then_with(|| a.name.cmp(&b.name))
    });
    devices
}

/// A device's sort group: connected first, then paired, then the rest.
fn group_of(device: &BtDevice) -> u8 {
    if device.connection == BtConnection::Connected {
        0
    } else if device.pairing == BtPairing::Paired {
        1
    } else {
        2
    }
}
