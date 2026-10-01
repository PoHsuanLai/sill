//! The state types (data only).

use keycap_protocol::Status;
use serde::{Deserialize, Serialize};

/// Where the connection to keycap stands. Failure and absence are states, not a silent default:
/// a machine without keycap is `NotInstalled`, one not yet heard from is `Unknown`. Crosses the
/// wire as `sill keycap status`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum KeycapState {
    /// Nothing known yet, or the unit was stopped on purpose.
    #[default]
    Unknown,
    /// The socket is up; `status` is keycap's latest word on its mode and keyboards.
    Connected { status: Status },
    /// The connection dropped or the unit is starting again; systemd is expected to finish it.
    Restarting,
    /// The unit failed (a crash loop that systemd gave up on).
    Failed,
    /// No keycap unit exists on this machine.
    NotInstalled,
}

/// What the person should be told about keycap. Everything else, including a restart that
/// recovers, is silent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attention {
    /// keycap is down for good: the keyboards are not remapped.
    Failed,
    /// Another remapper holds this keyboard, so keycap cannot.
    Conflict { device: String },
    /// The escape chord paused remapping; `Resume` ends it.
    Paused,
}
