//! The status tray: an SNI host with dbusmenu (plan hooks table, `use_tray`: "system-tray 0.8.9:
//! SNI host + dbusmenu; registers watcher if none").

#![allow(unused_variables, dead_code)] // Freeze stubs

mod model;

pub use model::{
    Availability, Check, Pixmap, Tray, TrayCommand, TrayEvent, TrayIcon, TrayItem, TrayItemId,
    TrayMenuEntry, TrayMenuKind, TrayStatus, reduce_tray,
};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// Start the tray host on `rt`, registering a `StatusNotifierWatcher` when none is on the bus.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Tray>, Control<TrayCommand>) {
    todo!("M1 svc-tray: system_tray::client::Client events -> TrayEvent -> reduce_tray")
}
