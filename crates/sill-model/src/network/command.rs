//! What the network service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::{Activating, Connectivity, Link};
use crate::units::Percent;

/// One change NetworkManager reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum NetworkEvent {
    /// The primary connection changed.
    Link(Link),
    /// The active access point's strength changed; ignored unless the link is Wi-Fi.
    Strength(Percent),
    Connectivity(Connectivity),
    /// `ActivatingConnection` changed.
    Activating(Activating),
    /// NetworkManager left the bus.
    Gone,
}

/// What the network service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkCommand {
    /// Re-read the primary connection and connectivity.
    Refresh,
}
