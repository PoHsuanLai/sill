//! The network's state.

use serde::{Deserialize, Serialize};

use crate::attempt::Attempts;
use crate::units::Percent;

/// What the bar's network item draws.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Network {
    pub link: Link,
    pub connectivity: Connectivity,
    /// The join attempts: one stamp per entry into `Connecting`, a new one per failed
    /// activation (the bar's Wi-Fi glyph, quire design/26 R4 and R6).
    #[serde(default)]
    pub join: Attempts,
}

/// The primary connection.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Link {
    #[default]
    Disconnected,
    /// Activating, not yet up.
    Connecting,
    Wired,
    Wifi {
        ssid: String,
        strength: Percent,
    },
    /// A VPN, a bridge, a modem: shown as connected without a signal.
    Other,
}

/// NetworkManager's `Connectivity`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Connectivity {
    #[default]
    Unknown,
    None,
    /// Behind a captive portal.
    Portal,
    Limited,
    Full,
}

impl Connectivity {
    /// NetworkManager's `NMConnectivityState` number; anything else is `Unknown`.
    pub fn from_nm(state: u32) -> Connectivity {
        match state {
            1 => Connectivity::None,
            2 => Connectivity::Portal,
            3 => Connectivity::Limited,
            4 => Connectivity::Full,
            _ => Connectivity::Unknown,
        }
    }
}

/// What NetworkManager's `ActivatingConnection` reports about a connection other than the
/// current primary: the primary's own activation already reaches the join through `Link` (a
/// primary connection can itself be seen `Connecting`), so this only carries a second, independent
/// connection trying to become primary, Wi-Fi only (the case the reference's
/// searching glyph is for).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activating {
    /// Nothing else is activating, its `Type` is not `802-11-wireless`, or it cannot be read.
    Idle,
    /// A Wi-Fi connection is activating (`State` 1).
    Joining,
}
