//! What Night Shift is told, what happens to it, and what it asks the files to do.

use super::{Backend, Manual, MinuteOfDay, NightLight, UnixSeconds, Warmth};

/// The time, as a decision needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Now {
    pub unix: UnixSeconds,
    /// The local time of day.
    pub minute: MinuteOfDay,
}

/// What the service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NightShiftCommand {
    /// Warm the screen now, until tomorrow's end of the night.
    TurnOnUntilTomorrow,
    /// Stop warming it: end a manual turn-on, else switch the night light off.
    TurnOff,
    /// How warm the screen gets.
    SetWarmth(Warmth),
    /// Who warms the screen (the displays' settings changed).
    Backend(Backend),
}

/// One thing that happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NightShiftEvent {
    /// casement's settings and the manual turn-on were read (again).
    Loaded {
        light: NightLight,
        manual: Option<Manual>,
    },
    /// casement's settings could not be read or written.
    Unreachable,
    /// The person asked.
    Told(NightShiftCommand, Now),
    /// The clock moved on: a manual turn-on may have run out.
    Tick(Now),
}

/// What the service must then do to the files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NightShiftEffect {
    /// Write casement's `[night_light]`.
    WriteLight(NightLight),
    /// Write the manual turn-on, or forget it.
    WriteManual(Option<Manual>),
}
