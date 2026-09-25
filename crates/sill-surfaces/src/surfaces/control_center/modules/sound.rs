//! The Sound module: a [`crate::surfaces::control_center::LevelModule`] with the volume glyph
//! (by level), the output device's name as its title, and the level. It reads no service: the
//! caller ([`super::Module`], the bar's volume dropdown) passes `Volume::level` and sends
//! `VolumeCommand::SetLevel` from `on_change`, so the same component serves anywhere a level is
//! shown. Choosing another output device needs a sink list and a `SetSink` the volume service
//! does not have; it is left for later (FINDINGS "M5 freeze").

use dioxus::prelude::*;
use ds::{Availability, Fraction, Icon};

use crate::surfaces::control_center::LevelModule;

/// The Sound module at `level` on `device` (`Volume::sink`).
#[component]
pub fn SoundModule(
    level: Fraction,
    device: String,
    on_change: EventHandler<Fraction>,
    #[props(default)] availability: Availability,
) -> Element {
    rsx! {
        LevelModule { glyph: sound_glyph(level), title: device, level, on_change, availability }
    }
}

/// The volume glyph by level (the bar's `volume_icon` thresholds, on thousandths).
pub(crate) fn sound_glyph(level: Fraction) -> Icon {
    match level.0 {
        0 => Icon::VolumeX,
        1..=333 => Icon::Volume,
        334..=666 => Icon::Volume1,
        _ => Icon::Volume2,
    }
}

#[cfg(test)]
mod tests {
    use super::sound_glyph;
    use ds::{Fraction, Icon};

    #[test]
    fn the_glyph_follows_the_level() {
        let cases = [
            (0, Icon::VolumeX),
            (100, Icon::Volume),
            (500, Icon::Volume1),
            (1000, Icon::Volume2),
        ];
        for (level, want) in cases {
            assert_eq!(sound_glyph(Fraction(level)), want, "{level}");
        }
    }
}
