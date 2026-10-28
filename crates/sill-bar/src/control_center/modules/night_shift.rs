//! The Night Shift row under the Display module's brightness (macOS's Night Shift): a switch that
//! warms the screen until tomorrow, a line that says what it is doing, and, while it warms or
//! will, the colour temperature from Less Warm to More Warm. It reads and commands the Night
//! Shift service; the schedule (times, "Always") is casement's night light, set in Settings.
//! Nothing is drawn while casement's settings cannot be reached.

use dioxus::prelude::*;
use ds::prelude::*;
use ds::root::common::Common;
use ds::root::pass_through::ExtraClass;
use sill_model::clock::{ClockHours, time_of_minute};
use sill_model::night_shift::{
    NightShift, NightShiftCommand, Shown, shown, warmth_at, warmth_fraction,
};
use sill_services::NightShiftService;
use sill_shell_kit::prelude::*;

use super::use_plate;
use sill_ui_kit::hooks::{use_control, use_hours, use_service};

/// The row's element id (a handle for tests and a person's stylesheet).
const ROW_ID: &str = "night-shift";

/// What the row shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Face {
    /// What Night Shift is doing, in a line.
    pub line: String,
    /// The switch.
    pub switch: Check,
    /// The warmth slider's place, while the screen is or will be warmed.
    pub warmth: Option<Fraction>,
}

/// The row for `night_shift`; none while casement's settings cannot be reached.
pub(crate) fn face_of(night_shift: &NightShift, hours: ClockHours) -> Option<Face> {
    let level = Fraction(warmth_fraction(night_shift.light.warmth));
    let (line, switch, warmth) = match shown(night_shift) {
        Shown::Unavailable => return None,
        Shown::Off => ("Off".to_owned(), Check::Off, None),
        Shown::OnUntilTomorrow => ("On until tomorrow".to_owned(), Check::On, Some(level)),
        Shown::On => ("On".to_owned(), Check::On, Some(level)),
        Shown::Scheduled { from, to } => (
            format!(
                "Scheduled, {} to {}",
                time_of_minute(from.minutes(), hours),
                time_of_minute(to.minutes(), hours)
            ),
            Check::Off,
            Some(level),
        ),
    };
    Some(Face {
        line,
        switch,
        warmth,
    })
}

/// The command a flip of the switch to `to` sends.
pub(crate) fn command_of(to: Check) -> NightShiftCommand {
    match to {
        Check::On => NightShiftCommand::TurnOnUntilTomorrow,
        Check::Off | Check::Mixed => NightShiftCommand::TurnOff,
    }
}

/// The Night Shift row.
#[component]
pub(crate) fn NightShiftRow() -> Element {
    let state = use_service::<NightShiftService>()();
    let control = use_control::<NightShiftService>();
    let Some(face) = face_of(&state, use_hours()) else {
        return rsx! {};
    };
    let warmth = control.clone();
    rsx! {
        ModulePanel {
            title: TextLine::from("Night Shift"),
            plate: use_plate(),
            common: Common {
                id: Some(ROW_ID.to_owned()),
                extra_class: ExtraClass::parse("sill-cc-night").ok(),
                ..Common::default()
            },
            trailing: rsx! {
                Toggle {
                    label: "Night Shift".to_owned(),
                    value: face.switch,
                            onchange: move |to: Check| {
                        // A stopped service drops it: the switch shows what it last published.
                        let _gone = control.send(command_of(to));
                    },
                }
            },
            div { class: "sill-cc-night-line", "{face.line}" }
            if let Some(level) = face.warmth {
                div { class: "sill-cc-night-warmth",
                    Slider {
                        label: "Colour Temperature".to_owned(),
                        value: level,
                                    onchange: move |to: Fraction| {
                            let _gone = warmth.send(NightShiftCommand::SetWarmth(warmth_at(to.0)));
                        },
                    }
                    div { class: "sill-cc-night-ends",
                        span { "Less Warm" }
                        span { "More Warm" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use ds::prelude::*;
    use sill_model::displays::NightWarmthMode;
    use sill_model::night_shift::{Manual, NightLight, NightShift, Schedule, UnixSeconds};

    use super::{ClockHours, Face, command_of, face_of};
    use sill_model::night_shift::NightShiftCommand;

    fn ready(schedule: Schedule, manual: Option<Manual>, backend: NightWarmthMode) -> NightShift {
        NightShift {
            reached: true,
            light: NightLight {
                schedule,
                ..NightLight::default()
            },
            manual,
            backend,
        }
    }

    fn face(line: &str, switch: Check, warm: bool) -> Option<Face> {
        Some(Face {
            line: line.to_owned(),
            switch,
            // casement's default 3400 K sits at 815 thousandths of the slider.
            warmth: warm.then_some(Fraction(815)),
        })
    }

    #[test]
    fn the_row_says_what_night_shift_is_doing() {
        let by_hand = Manual {
            until: UnixSeconds(1),
            restore: NightLight::default(),
        };
        let cases = [
            ("unreachable", NightShift::default(), None),
            (
                "off",
                ready(Schedule::Off, None, NightWarmthMode::Off),
                face("Off", Check::Off, false),
            ),
            (
                "until tomorrow",
                ready(Schedule::Always, Some(by_hand), NightWarmthMode::Off),
                face("On until tomorrow", Check::On, true),
            ),
            (
                "all day",
                ready(Schedule::Always, None, NightWarmthMode::Off),
                face("On", Check::On, true),
            ),
            (
                "on a schedule",
                ready(Schedule::Scheduled, None, NightWarmthMode::Off),
                face("Scheduled, 22:00 to 07:00", Check::Off, true),
            ),
        ];
        for (name, state, want) in cases {
            assert_eq!(face_of(&state, ClockHours::H24), want, "{name}");
        }
    }

    #[test]
    fn the_switch_asks_for_until_tomorrow_or_off() {
        assert_eq!(
            command_of(Check::On),
            NightShiftCommand::TurnOnUntilTomorrow
        );
        assert_eq!(command_of(Check::Off), NightShiftCommand::TurnOff);
    }
}
