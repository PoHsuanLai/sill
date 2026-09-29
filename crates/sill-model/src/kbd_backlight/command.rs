//! What the kbd backlight service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::KbdLevel;
use crate::units::Percent;

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

/// What the keyboard backlight service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KbdCommand {
    /// Set the backlight to the step nearest `Percent` of its range; dropped while absent.
    Set(Percent),
    /// Re-read the backlight (after a resume).
    Refresh,
}
