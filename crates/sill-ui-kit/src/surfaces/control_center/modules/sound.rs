//! The Sound module: a [`crate::surfaces::control_center::LevelModule`] headed by the bar item's
//! volume glyph, the output device's name as its title, and the level, whose speaker draws the
//! same `VolumeState` (the module and the bar read one source). It reads no service:
//! the caller ([`super::Module`], the bar's volume dropdown) passes `volume_glyph(&volume)` and
//! `Volume::level` and sends `VolumeCommand::SetLevel` from `on_change`, so the same component
//! serves anywhere a level is shown. Choosing another output device needs a sink list and a
//! `SetSink` the volume service does not have; it is left for later.

use dioxus::prelude::*;
use ds::{Availability, Fraction, StatusState, VolumeState};

use crate::surfaces::control_center::LevelModule;

/// The Sound module at `level` on `device` (`Volume::sink`), its glyphs drawing `volume`.
#[component]
pub fn SoundModule(
    volume: VolumeState,
    level: Fraction,
    device: String,
    on_change: EventHandler<Fraction>,
    #[props(default)] availability: Availability,
) -> Element {
    rsx! {
        LevelModule {
            glyph: StatusState::Volume(volume),
            level_glyph: volume,
            title: device,
            level,
            on_change,
            availability,
        }
    }
}
