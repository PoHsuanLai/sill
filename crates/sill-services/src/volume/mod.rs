//! The output volume: libpulse over pipewire-pulse on a threaded mainloop, following the
//! default sink (plan hooks table, `use_volume`).

#![allow(unused_variables, dead_code)] // Freeze stubs

mod model;

pub use model::{Mute, Step, Volume, VolumeCommand, VolumeEvent, reduce_volume, stepped};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// Start the volume thread (libpulse's threaded mainloop; its callbacks feed `reduce_volume`
/// through `rt`). Without a pulse server the receiver stays at `Volume::default()` (absent).
pub fn spawn(rt: &Handle) -> (watch::Receiver<Volume>, Control<VolumeCommand>) {
    todo!("M1 svc-dbus: libpulse-binding threaded mainloop, subscribe sink|server -> reduce_volume")
}
