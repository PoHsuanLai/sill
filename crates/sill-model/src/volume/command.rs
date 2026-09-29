//! What the volume service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::Mute;
use crate::units::Percent;

/// A volume key's direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Up,
    Down,
}

/// What the volume service can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum VolumeCommand {
    SetLevel(Percent),
    /// One key press: 5 points, clamped to 0..=100 (the OSD's step).
    Step(Step),
    SetMute(Mute),
    ToggleMute,
}

/// One change the pulse server reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum VolumeEvent {
    /// The default sink is now this one (or it changed wholesale).
    Sink {
        sink: String,
        level: Percent,
        mute: Mute,
    },
    Level(Percent),
    Mute(Mute),
    /// There is no default sink any more.
    SinkGone,
    /// The server went away.
    ServerGone,
}
