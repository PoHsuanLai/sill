//! The status tray: an SNI host with dbusmenu (system-tray 0.8.9: SNI host + dbusmenu; registers a
//! watcher if none).
//!
//! [`spawn`] and [`host`] do the effect (a `system_tray::client::Client` on the session bus);
//! [`icon`] turns a resolved [`TrayIcon`] into the art the bar draws (PNG bytes or a theme
//! file). Neither reaches `dioxus`, `ds` or `shell-host` (`scripts/check-boundary.sh`): the bar
//! reads this module only through [`Tray`], [`TrayCommand`] and [`icon_art`], via
//! `sill-surfaces::hooks::use_tray`.

mod host;
mod icon;
mod model;

pub use icon::{IconArt, icon_art};
pub use model::{
    Availability, Check, Pixmap, ScrollAxis, ScrollDelta, Tray, TrayCommand, TrayEvent, TrayIcon,
    TrayItem, TrayItemId, TrayMenuEntry, TrayMenuKind, TrayStatus, reduce_tray,
};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// Start the tray host on `rt`, registering a `StatusNotifierWatcher` when none is on the bus
/// (`system_tray::client::Client::new`'s own `StatusNotifierWatcher::attach_to`: it tries to
/// own `org.kde.StatusNotifierWatcher` and quietly defers when a desktop already does). A
/// missing session bus leaves the receiver at `Tray::default()` — starting a service never
/// fails the shell (`error.rs`).
pub fn spawn(rt: &Handle) -> (watch::Receiver<Tray>, Control<TrayCommand>) {
    let (tray_tx, tray_rx) = watch::channel(Tray::default());
    let (control, commands) = Control::channel();
    rt.spawn(host::run(tray_tx, commands));
    (tray_rx, control)
}
