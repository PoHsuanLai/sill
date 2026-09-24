//! The battery: UPower's display device over the system bus (plan hooks table, `use_battery`:
//! "upower_dbus git: display device streams").

#![allow(unused_variables, dead_code)] // Freeze stubs

mod model;

pub use model::{Battery, BatteryEvent, Charge, Remaining, reduce_battery};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// What the battery service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryCommand {
    /// Re-read every property (after a resume).
    Refresh,
}

/// Start the battery task on `rt`. Without UPower the receiver stays at `Battery::default()`
/// (absent), and the bar hides the item.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Battery>, Control<BatteryCommand>) {
    todo!(
        "M1 svc-dbus: upower_dbus DisplayDevice property streams -> BatteryEvent -> reduce_battery"
    )
}
