//! What can happen to the connection, and what the person can ask of it.

use keycap_protocol::Status;
use serde::{Deserialize, Serialize};

use crate::keyboard::UserOverrides;

/// What a person can ask of keycap from outside the daemon (`sill keycap retry` and `resume`):
/// the part of [`KeycapCommand`] that crosses the IPC wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeycapAsk {
    /// Reset the unit's failure and start it again.
    Retry,
    /// End the pause the escape chord caused.
    Resume,
}

/// What the keycap service can be told: a person's [`KeycapAsk`], and the settings' overrides.
/// Not on the wire: the overrides come only from `keyboard.overrides` in the settings file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeycapCommand {
    /// Reset the unit's failure and start it again.
    Retry,
    /// End the pause the escape chord caused.
    Resume,
    /// The person's per-app profiles from the settings; replaces the ones the service has and
    /// sends the focus again when the window's profile changes.
    Overrides(UserOverrides),
}

impl From<KeycapAsk> for KeycapCommand {
    fn from(ask: KeycapAsk) -> Self {
        match ask {
            KeycapAsk::Retry => KeycapCommand::Retry,
            KeycapAsk::Resume => KeycapCommand::Resume,
        }
    }
}

/// The systemd unit's state, as the service reads it from the manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitState {
    Active,
    /// Starting, including systemd's automatic restart delay.
    Activating,
    Failed,
    /// Stopped without a failure.
    Inactive,
    /// The manager has no such unit.
    NotFound,
}

/// One fact about keycap, from the socket or from systemd.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeycapEvent {
    /// The first packet on a new connection.
    Hello(Status),
    /// keycap's status changed.
    Status(Status),
    /// The socket closed.
    Disconnected,
    UnitState(UnitState),
}
