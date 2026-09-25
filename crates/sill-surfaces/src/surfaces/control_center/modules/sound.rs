//! The Sound module: a [`crate::surfaces::control_center::LevelModule`] with the volume glyph
//! (by level), the output device's name as its title, and the level. It reads no service: the
//! caller ([`super::Module`], the bar's volume dropdown) passes `Volume::level` and sends
//! `VolumeCommand::SetLevel` from `on_change`, so the same component serves anywhere a level is
//! shown. Choosing another output device needs a sink list and a `SetSink` the volume service
//! does not have; it is left for later (FINDINGS "M5 freeze").

use dioxus::prelude::*;
use ds::{Availability, Fraction};

/// The Sound module at `level` on `device` (`Volume::sink`).
#[component]
pub fn SoundModule(
    level: Fraction,
    device: String,
    on_change: EventHandler<Fraction>,
    #[props(default)] availability: Availability,
) -> Element {
    let _ = (level, device, on_change, availability);
    todo!("M5 fill: control-center (c)")
}
