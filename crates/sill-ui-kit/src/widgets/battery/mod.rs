//! The Batteries widget's provider (design/23-WIDGETS.md section 4.1): this
//! machine's battery (UPower) and every connected Bluetooth device that reports one (BlueZ
//! `Battery1`), as quire's `BatteryEntry`. quire's `BatteryWidget` draws them (one ring with the
//! hero figure, the 2 x 2 grid, the row; each ring holds the device's filled glyph and fills on
//! the host's wake); sill only reads the services. Nothing to show (a desktop with no battery
//! and no devices): `Absent("No batteries")`.

mod io;
mod provider;
#[cfg(test)]
mod tests;

pub use io::BatteryCard;
pub use provider::{BatteryOwner, BatteryRow, battery_entry, battery_rows, cell_of, device_of};
