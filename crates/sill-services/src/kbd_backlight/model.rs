//! The keyboard backlight's state, its one reducer, and the percent-to-step mapping.

use serde::{Deserialize, Serialize};

use sill_model::units::{Percent, Presence};

/// The keyboard backlight, as the control center draws it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct KbdBacklight {
    /// Whether the machine has one (UPower answered with a range above 0).
    pub presence: Presence,
    pub level: KbdLevel,
}

/// The backlight's raw step and its range: most keyboards have two or three steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct KbdLevel {
    pub value: u32,
    pub max: u32,
}

impl KbdLevel {
    /// The step as the nearest whole percentage of the range (0 with no range).
    pub fn percent(self) -> Percent {
        match self.max {
            0 => Percent(0),
            max => {
                let value = u64::from(self.value.min(max));
                let rounded = (value * 100 + u64::from(max) / 2) / u64::from(max);
                Percent(u16::try_from(rounded).unwrap_or(100))
            }
        }
    }
}

/// One change UPower reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum KbdEvent {
    /// A backlight answered: its step and range.
    Found(KbdLevel),
    /// `BrightnessChanged`: the new step (a key, another app, or this shell's own set).
    Brightness(u32),
    /// No backlight, or UPower left the bus.
    Gone,
}

/// `backlight` after `event`. A step while absent is dropped: `Found` always comes first.
pub fn step(backlight: KbdBacklight, event: KbdEvent) -> KbdBacklight {
    match (backlight.presence, event) {
        (_, KbdEvent::Found(level)) => KbdBacklight {
            presence: Presence::Present,
            level: KbdLevel {
                value: level.value.min(level.max),
                max: level.max,
            },
        },
        (Presence::Present, KbdEvent::Brightness(value)) => KbdBacklight {
            level: KbdLevel {
                value: value.min(backlight.level.max),
                ..backlight.level
            },
            ..backlight
        },
        (Presence::Absent, KbdEvent::Brightness(_)) => backlight,
        (_, KbdEvent::Gone) => KbdBacklight::default(),
    }
}

/// The step nearest `percent` of a range of `max` steps: what a drag to `percent` sets.
pub fn step_for(percent: Percent, max: u32) -> u32 {
    let percent = u64::from(percent.0.min(100));
    let step = (percent * u64::from(max) + 50) / 100;
    u32::try_from(step).unwrap_or(max)
}

#[cfg(test)]
mod tests {
    use super::{KbdBacklight, KbdEvent, KbdLevel, step, step_for};
    use sill_model::units::{Percent, Presence};

    fn at(value: u32, max: u32) -> KbdLevel {
        KbdLevel { value, max }
    }

    fn present(value: u32, max: u32) -> KbdBacklight {
        KbdBacklight {
            presence: Presence::Present,
            level: at(value, max),
        }
    }

    #[test]
    fn the_backlight_follows_upower() {
        // (name, before, event, after)
        let cases = [
            (
                "found",
                KbdBacklight::default(),
                KbdEvent::Found(at(1, 3)),
                present(1, 3),
            ),
            (
                "a key press",
                present(1, 3),
                KbdEvent::Brightness(2),
                present(2, 3),
            ),
            (
                "past the range is the top",
                present(1, 3),
                KbdEvent::Brightness(9),
                present(3, 3),
            ),
            (
                "a step before any backlight",
                KbdBacklight::default(),
                KbdEvent::Brightness(2),
                KbdBacklight::default(),
            ),
            (
                "gone",
                present(2, 3),
                KbdEvent::Gone,
                KbdBacklight::default(),
            ),
        ];
        for (name, before, event, after) in cases {
            assert_eq!(step(before, event), after, "{name}");
        }
    }

    #[test]
    fn steps_and_percentages_round_to_the_nearest() {
        // (value, max, percent)
        let shown = [
            (0, 3, 0),
            (1, 3, 33),
            (2, 3, 67),
            (3, 3, 100),
            (1, 2, 50),
            (0, 0, 0),
        ];
        for (value, max, percent) in shown {
            assert_eq!(at(value, max).percent(), Percent(percent), "{value}/{max}");
        }
        // (percent, max, step)
        let set = [
            (0, 3, 0),
            (20, 3, 1),
            (40, 3, 1),
            (50, 3, 2),
            (100, 3, 3),
            (70, 2, 1),
            (80, 2, 2),
            (150, 3, 3),
        ];
        for (percent, max, step) in set {
            assert_eq!(step_for(Percent(percent), max), step, "{percent}% of {max}");
        }
    }
}
