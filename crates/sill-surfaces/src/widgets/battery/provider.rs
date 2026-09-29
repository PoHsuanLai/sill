//! The Batteries widget's provider (design/23-WIDGETS.md section 4.1): this
//! machine's battery (UPower) and every connected Bluetooth device that reports one (BlueZ
//! `Battery1`), as quire's `BatteryEntry`. quire's `BatteryWidget` draws them (one ring with the
//! hero figure, the 2 x 2 grid, the row; each ring holds the device's filled glyph and fills on
//! the host's wake); sill only reads the services. Nothing to show (a desktop with no battery
//! and no devices): `Absent("No batteries")`.

use ds::{BatteryCell, BatteryEntry, Device, Fraction, RingMark};
use sill_model::battery::{Battery, Charge};
use sill_model::bluetooth::{Bluetooth, BtBattery, BtConnection, BtKind};
use sill_model::units::{Percent, Presence};

/// A device's filled glyph (quire's `DeviceGlyph`, twelve devices): the battery rings' glyph
/// in the Batteries widget and the control center's Battery module.
pub fn device_of(kind: BtKind) -> ds::Device {
    match kind {
        BtKind::Headphones | BtKind::Headset => ds::Device::Headphones,
        BtKind::Speaker => ds::Device::Speaker,
        BtKind::Keyboard => ds::Device::Keyboard,
        BtKind::Mouse => ds::Device::Mouse,
        BtKind::Gamepad => ds::Device::Gamepad,
        BtKind::Phone => ds::Device::Phone,
        BtKind::Computer => ds::Device::Laptop,
        BtKind::Other => ds::Device::Other,
    }
}

/// Whose battery a row is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BatteryOwner {
    ThisMachine,
    Device(BtKind),
}

/// One row.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BatteryRow {
    pub owner: BatteryOwner,
    /// "This Mac"'s stand-in is the host name's first label, else "This Computer"; a device's
    /// name is its alias.
    pub name: String,
    pub level: Percent,
    /// This machine's charge state; a device's is `Unknown` (BlueZ does not say).
    pub charge: Charge,
}

/// This machine's row first when it has a battery, then each connected device with a level,
/// by name. `machine` is the name the machine's row takes.
pub fn battery_rows(battery: &Battery, bluetooth: &Bluetooth, machine: &str) -> Vec<BatteryRow> {
    let this = match battery.presence {
        Presence::Present => Some(BatteryRow {
            owner: BatteryOwner::ThisMachine,
            name: machine.to_owned(),
            level: battery.level,
            charge: battery.charge,
        }),
        Presence::Absent => None,
    };
    let mut devices: Vec<BatteryRow> = bluetooth
        .devices
        .iter()
        .filter(|device| device.connection == BtConnection::Connected)
        .filter_map(|device| match device.battery {
            BtBattery::Level(level) => Some(BatteryRow {
                owner: BatteryOwner::Device(device.kind),
                name: device.name.clone(),
                level,
                charge: Charge::Unknown,
            }),
            BtBattery::Unknown => None,
        })
        .collect();
    devices.sort_by_key(|row| row.name.to_lowercase());
    this.into_iter().chain(devices).collect()
}

/// The machine row's name: the host name's first label, else "This Computer".
pub(super) fn machine_name(host: &str) -> String {
    match host
        .trim()
        .split('.')
        .next()
        .filter(|label| !label.is_empty())
    {
        Some(label) => label.to_owned(),
        None => "This Computer".to_owned(),
    }
}

/// A row as quire's cell: this machine is a laptop (it has a battery), a device its kind's
/// glyph; the level in permille, the bolt while it charges.
pub fn cell_of(row: &BatteryRow) -> BatteryCell {
    let device = match row.owner {
        BatteryOwner::ThisMachine => Device::Laptop,
        BatteryOwner::Device(kind) => device_of(kind),
    };
    let mark = match row.charge {
        Charge::Charging => RingMark::Charging,
        Charge::Unknown | Charge::Discharging | Charge::Empty | Charge::Full | Charge::Idle => {
            RingMark::Plain
        }
    };
    BatteryCell {
        name: row.name.clone(),
        device,
        level: Fraction(row.level.0.min(100) * 10),
        mark,
    }
}

/// The widget's entry for `rows`.
pub fn battery_entry(rows: &[BatteryRow]) -> BatteryEntry {
    match rows {
        [] => BatteryEntry::Absent("No batteries".to_owned()),
        rows => BatteryEntry::Devices(rows.iter().map(cell_of).collect()),
    }
}
