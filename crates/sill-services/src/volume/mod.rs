//! The output volume: libpulse over pipewire-pulse on a threaded mainloop, following the
//! default sink (plan hooks table, `use_volume`).

mod live;
mod model;

pub use model::{Mute, Step, Volume, VolumeCommand, VolumeEvent, reduce_volume, stepped};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// Start the volume thread (libpulse's threaded mainloop, on its own OS thread via
/// `Handle::spawn_blocking`; its callbacks feed `reduce_volume`). Without a pulse server the
/// receiver stays at `Volume::default()` (absent). The mainloop thread exits once `commands`
/// closes, which happens when every clone of the returned `Control` has dropped.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Volume>, Control<VolumeCommand>) {
    let (tx, rx) = watch::channel(Volume::default());
    let (control, commands) = Control::channel();
    rt.spawn_blocking(move || live::run(tx, commands));
    (rx, control)
}
