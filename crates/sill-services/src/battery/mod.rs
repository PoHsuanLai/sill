//! The battery: UPower's display device over the system bus.

mod live;
mod model;

pub use model::{Battery, BatteryEvent, Charge, Remaining, reduce_battery};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::env::Bus;

/// What the battery service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryCommand {
    /// Re-read every property (after a resume).
    Refresh,
}

/// Start the battery task on `rt`. Without UPower the receiver stays at `Battery::default()`
/// (absent), and the bar hides the item.
pub fn spawn(rt: &Handle, bus: Bus) -> (watch::Receiver<Battery>, Control<BatteryCommand>) {
    let (tx, rx) = watch::channel(Battery::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(bus, tx, commands));
    (rx, control)
}
