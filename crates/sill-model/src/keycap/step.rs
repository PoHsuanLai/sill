//! The pure transition and the decision to tell the person.

use keycap_protocol::{Grab, Mode, Status};

use super::command::{KeycapEvent, UnitState};
use super::model::{Attention, KeycapState};

/// `state` after `event`. The socket decides `Connected` and a drop out of it; the unit decides
/// everything else, so a drop followed by `Failed` is `Failed` and one followed by `Active` and a
/// new `Hello` is back to `Connected` without anyone being told.
pub fn step(state: KeycapState, event: KeycapEvent) -> KeycapState {
    match (state, event) {
        (_, KeycapEvent::Hello(status) | KeycapEvent::Status(status)) => {
            KeycapState::Connected { status }
        }
        (KeycapState::Connected { .. }, KeycapEvent::Disconnected) => KeycapState::Restarting,
        (state, KeycapEvent::Disconnected) => state,
        (_, KeycapEvent::UnitState(UnitState::Failed)) => KeycapState::Failed,
        (_, KeycapEvent::UnitState(UnitState::NotFound)) => KeycapState::NotInstalled,
        (_, KeycapEvent::UnitState(UnitState::Inactive)) => KeycapState::Unknown,
        (_, KeycapEvent::UnitState(UnitState::Activating)) => KeycapState::Restarting,
        (
            state @ (KeycapState::Connected { .. } | KeycapState::Unknown),
            KeycapEvent::UnitState(UnitState::Active),
        ) => state,
        (_, KeycapEvent::UnitState(UnitState::Active)) => KeycapState::Restarting,
    }
}

/// What the person should be told, if anything. A conflict outranks a pause: it needs them to
/// act on another program, a pause only a key chord.
pub fn attention(state: &KeycapState) -> Option<Attention> {
    match state {
        KeycapState::Failed => Some(Attention::Failed),
        KeycapState::Connected { status } => conflict(status).or_else(|| paused(status)),
        KeycapState::Unknown | KeycapState::Restarting | KeycapState::NotInstalled => None,
    }
}

fn conflict(status: &Status) -> Option<Attention> {
    status
        .keyboards
        .iter()
        .find(|keyboard| keyboard.grab == Grab::Conflict)
        .map(|keyboard| Attention::Conflict {
            device: keyboard.info.name.clone(),
        })
}

fn paused(status: &Status) -> Option<Attention> {
    (status.mode == Mode::Paused).then_some(Attention::Paused)
}
