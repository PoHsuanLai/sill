//! The Bluetooth tile: the adapter's radio as a quire `Toggle`,
//! [`crate::surfaces::control_center::bluetooth_status`] as its line, and a chevron to the
//! device list (`Detail::Bluetooth`). Reads `use_bluetooth`; sends `BluetoothCommand::Power`.

use dioxus::prelude::*;

use crate::surfaces::control_center::page::Detail;

/// The Bluetooth tile.
#[component]
pub fn BluetoothModule(on_detail: EventHandler<Detail>) -> Element {
    let _ = on_detail;
    todo!("M5 fill: control-center (c)")
}
