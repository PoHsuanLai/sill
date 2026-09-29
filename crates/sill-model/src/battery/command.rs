//! What the battery service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::Charge;
use crate::units::{Percent, Presence, Seconds};

/// One change UPower reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum BatteryEvent {
    Presence(Presence),
    Level(Percent),
    Charge(Charge),
    TimeToEmpty(Seconds),
    TimeToFull(Seconds),
    /// UPower left the bus: back to the default, absent.
    Gone,
}

/// What the battery service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryCommand {
    /// Re-read every property (after a resume).
    Refresh,
}
