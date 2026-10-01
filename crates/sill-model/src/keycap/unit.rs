//! Reading keycap's systemd unit: its `ActiveState` as a [`UnitState`], and when to ask.

use std::time::Duration;

use super::command::UnitState;
use super::model::KeycapState;

/// How long keycap may be unreachable before systemd is asked why: a restart takes a fraction
/// of a second and is nobody's business.
pub const UNIT_GRACE: Duration = Duration::from_secs(3);

/// How often systemd is asked again while keycap stays unreachable.
pub const UNIT_RECHECK: Duration = Duration::from_secs(5);

/// The unit's `ActiveState` as the manager spells it; `None` for a word this does not know, so
/// the state is left as it was rather than guessed.
pub fn unit_state(active_state: &str) -> Option<UnitState> {
    match active_state {
        "active" | "reloading" => Some(UnitState::Active),
        "activating" | "deactivating" => Some(UnitState::Activating),
        "failed" | "maintenance" => Some(UnitState::Failed),
        "inactive" => Some(UnitState::Inactive),
        _ => None,
    }
}

/// Whether the unit is worth asking about in `state`: while the socket is down, until systemd
/// says there is no such unit. A live connection already says keycap runs.
pub fn watching_unit(state: &KeycapState) -> bool {
    match state {
        KeycapState::Unknown | KeycapState::Restarting | KeycapState::Failed => true,
        KeycapState::Connected { .. } | KeycapState::NotInstalled => false,
    }
}
