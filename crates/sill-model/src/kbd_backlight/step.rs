//! The kbd backlight state's pure transition.

use super::{KbdBacklight, KbdEvent, KbdLevel};
use crate::units::{Percent, Presence};

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
