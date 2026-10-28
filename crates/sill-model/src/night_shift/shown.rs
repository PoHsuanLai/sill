//! What Night Shift looks like to the person, and the warmth slider's scale. Pure.

use super::{MinuteOfDay, NightShift, Schedule, Warmth};

/// What the Night Shift switch says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shown {
    /// casement's settings could not be reached: the switch does nothing.
    Unavailable,
    Off,
    /// Turned on by hand, until the night ends.
    OnUntilTomorrow,
    /// On all day (set in Settings).
    On,
    /// On a schedule (set in Settings): the switch is off until it begins.
    Scheduled {
        from: MinuteOfDay,
        to: MinuteOfDay,
    },
}

/// The switch's state for `night_shift`.
pub fn shown(night_shift: &NightShift) -> Shown {
    match (
        night_shift.reached,
        night_shift.manual,
        night_shift.light.schedule,
    ) {
        (false, _, _) => Shown::Unavailable,
        (true, Some(_), _) => Shown::OnUntilTomorrow,
        (true, None, Schedule::Always) => Shown::On,
        (true, None, Schedule::Scheduled) => Shown::Scheduled {
            from: night_shift.light.from,
            to: night_shift.light.to,
        },
        (true, None, Schedule::Off) => Shown::Off,
    }
}

/// The slider's "Less Warm" end, in kelvin.
const LESS_WARM: u32 = 6500;
/// The slider's "More Warm" end, in kelvin.
const MORE_WARM: u32 = 2700;

/// Where `warmth` lies on the slider, in thousandths from "Less Warm" to "More Warm"; a warmth
/// outside the slider's range sits at its nearer end.
pub fn warmth_fraction(warmth: Warmth) -> u16 {
    let kelvin = u32::from(warmth.kelvin()).clamp(MORE_WARM, LESS_WARM);
    let thousandths = (LESS_WARM - kelvin) * 1000 / (LESS_WARM - MORE_WARM);
    u16::try_from(thousandths).unwrap_or(1000)
}

/// The warmth at `thousandths` of the slider.
pub fn warmth_at(thousandths: u16) -> Warmth {
    let along = u32::from(thousandths.min(1000));
    Warmth::clamped(LESS_WARM - along * (LESS_WARM - MORE_WARM) / 1000)
}
