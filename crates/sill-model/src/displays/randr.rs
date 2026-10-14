//! The compositor's own view of its outputs (over `zwlr_output_manager_v1`): names, the current
//! mode, the physical size it was told, and the scale.

use serde::{Deserialize, Serialize};

use super::edid::{Millimetres, Mode};
use super::scale::DisplayScale;
use crate::ids::OutputName;

/// Whether the compositor is driving an output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enablement {
    #[default]
    Enabled,
    Disabled,
}

/// One output as the compositor reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Head {
    /// The connector name (`HDMI-A-2`).
    pub name: OutputName,
    pub enablement: Enablement,
    pub make: String,
    pub model: String,
    /// `None` when the compositor reports 0 x 0.
    pub physical: Option<Millimetres>,
    /// The mode marked `current`.
    pub current: Option<Mode>,
    pub scale: DisplayScale,
}

impl Default for Head {
    /// A nameless, enabled head at scale 1 with nothing known: what a head starts as.
    fn default() -> Self {
        Head {
            name: OutputName(String::new()),
            enablement: Enablement::Enabled,
            make: String::new(),
            model: String::new(),
            physical: None,
            current: None,
            scale: DisplayScale::ONE,
        }
    }
}
