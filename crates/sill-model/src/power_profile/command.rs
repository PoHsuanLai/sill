//! What the power profile service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::{Profile, ProfileDegraded};

/// One change the daemon reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ProfileEvent {
    /// The daemon answered.
    Found,
    Active(Profile),
    Offered(Vec<Profile>),
    Degraded(ProfileDegraded),
    /// The daemon left the bus.
    Gone,
}

/// What the power profile service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileCommand {
    /// Set `ActiveProfile`; one the daemon does not offer is dropped.
    Set(Profile),
    /// Re-read every property.
    Refresh,
}
