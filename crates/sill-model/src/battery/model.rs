//! The battery's state.

use crate::units::{Minutes, Percent, Presence, Seconds};
use serde::{Deserialize, Serialize};

/// UPower's display device, as the bar draws it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Battery {
    /// `IsPresent`: a desktop has no battery, and the bar hides the item.
    pub presence: Presence,
    /// `Percentage`, rounded.
    pub level: Percent,
    pub charge: Charge,
    /// `TimeToEmpty`; zero while unknown.
    pub time_to_empty: Seconds,
    /// `TimeToFull`; zero while unknown.
    pub time_to_full: Seconds,
}

/// UPower's `State`, folded to what the icon distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Charge {
    #[default]
    Unknown,
    Charging,
    Discharging,
    Empty,
    Full,
    /// Plugged in and not charging (UPower's pending-charge and pending-discharge).
    Idle,
}

impl Charge {
    /// UPower's `State` number (0 unknown, 1 charging, 2 discharging, 3 empty, 4 fully
    /// charged, 5 pending charge, 6 pending discharge); anything else is `Unknown`.
    pub fn from_upower(state: u32) -> Charge {
        match state {
            1 => Charge::Charging,
            2 => Charge::Discharging,
            3 => Charge::Empty,
            4 => Charge::Full,
            5 | 6 => Charge::Idle,
            _ => Charge::Unknown,
        }
    }
}

/// How long the battery has left, in the direction it is going.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Remaining {
    Unknown,
    ToEmpty(Minutes),
    ToFull(Minutes),
}

impl Battery {
    /// The time left in the direction the charge is going; `Unknown` when UPower has no
    /// estimate for that direction.
    pub fn remaining(&self) -> Remaining {
        let minutes = |s: Seconds| Minutes(u32::try_from(s.0 / 60).unwrap_or(u32::MAX));
        match self.charge {
            Charge::Discharging if self.time_to_empty.0 > 0 => {
                Remaining::ToEmpty(minutes(self.time_to_empty))
            }
            Charge::Charging if self.time_to_full.0 > 0 => {
                Remaining::ToFull(minutes(self.time_to_full))
            }
            _ => Remaining::Unknown,
        }
    }
}
