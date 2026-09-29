//! The volume's state.

use serde::{Deserialize, Serialize};

use crate::units::{Percent, Presence};

/// The default sink, as the bar's volume item and the OSD draw it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Volume {
    /// Whether there is a default sink at all.
    pub presence: Presence,
    /// The sink's volume, averaged over channels; can exceed 100.
    pub level: Percent,
    pub mute: Mute,
    /// The sink's human description ("Built-in Audio Analog Stereo").
    pub sink: String,
}

/// Whether the sink is muted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mute {
    #[default]
    Audible,
    Muted,
}
