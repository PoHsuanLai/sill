//! Bluetooth: BlueZ's adapter and devices, as the control center lists them.

mod command;
mod kind;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{BluetoothCommand, BluetoothEvent};
pub use kind::device_kind;
pub use model::{
    Bluetooth, BtAddress, BtBattery, BtConnection, BtDevice, BtKind, BtPairing, BtPower,
    listed_devices,
};
pub use step::step;
