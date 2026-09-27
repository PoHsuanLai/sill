//! The Keyboard Brightness module (design/26 5.2.7 G25): a
//! [`crate::surfaces::control_center::LevelModule`] with the `Keyboard` glyph and quire's
//! `LevelGlyph::KeyboardBrightness` (a keyboard under a rising sun whose rays follow the level),
//! on UPower's keyboard backlight. Absent on a machine without one. A drag sends
//! `KbdCommand::Set`; the service sets the nearest step and the level follows its report, the
//! laptop's own brightness keys included.

use dioxus::prelude::*;
use ds::{Fraction, Icon, LevelGlyph};
use sill_services::{KbdBacklight, KbdCommand, Presence};

use super::wired::{fraction_of, percent_of};
use crate::hooks::{use_kbd_backlight, use_services};
use crate::surfaces::control_center::LevelModule;

/// The Keyboard Brightness module, or nothing without a backlight.
#[component]
pub fn KeyboardModule() -> Element {
    let backlight = use_kbd_backlight()();
    let control = use_services().kbd_backlight_control;
    let Some(level) = keyboard_level(backlight) else {
        return rsx! {};
    };
    rsx! {
        LevelModule {
            glyph: Icon::Keyboard,
            level_glyph: LevelGlyph::KeyboardBrightness,
            title: "Keyboard Brightness".to_owned(),
            level,
            on_change: move |to: Fraction| {
                // A stopped service drops it: the level shows the step it last published.
                let _gone = control.send(KbdCommand::Set(percent_of(to)));
            },
        }
    }
}

/// The level the module shows, `None` without a backlight.
pub(crate) fn keyboard_level(backlight: KbdBacklight) -> Option<Fraction> {
    match backlight.presence {
        Presence::Present => Some(fraction_of(backlight.level.percent())),
        Presence::Absent => None,
    }
}

#[cfg(test)]
mod tests {
    use super::keyboard_level;
    use ds::Fraction;
    use sill_services::{KbdBacklight, KbdLevel, Presence};

    #[test]
    fn the_module_shows_only_where_there_is_a_backlight() {
        let at = |presence, value| KbdBacklight {
            presence,
            level: KbdLevel { value, max: 3 },
        };
        let cases = [
            (at(Presence::Present, 2), Some(Fraction(670))),
            (at(Presence::Present, 0), Some(Fraction(0))),
            (at(Presence::Absent, 0), None),
        ];
        for (backlight, want) in cases {
            assert_eq!(keyboard_level(backlight), want, "{backlight:?}");
        }
    }
}
