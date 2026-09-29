//! The keyboard backlight's state.

use serde::{Deserialize, Serialize};

use crate::units::{Percent, Presence};

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
