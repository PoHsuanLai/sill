//! The thumbnail's machine: pure, table-tested; the surface feeds it the service's current shot,
//! the pointer, the gestures, the drag's outcome and quire's timers, and carries out what it
//! returns.
//!
//! - A new current shot shows (`Show`) and starts the hold (`StartHold` with a fresh token); a
//!   newer one while shown or leaving replaces it the same way.
//! - The pointer on the card pauses the hold (the token moves on, so the running timer's
//!   `HoldElapsed` is stale); leaving it starts `rest_ms` more. A drag out pauses it the same
//!   way until its outcome comes back.
//! - The hold running out with the pointer away, a swipe, a click, a drop and Delete each leave:
//!   `Leave(id, why)` (the surface tells the service) and the state is `Leaving` while the exit
//!   plays, the card's own (`ThumbExit::Card`) or, after a swipe, its row's (`ThumbExit::Row`);
//!   `ExitSettled` hides (`Hide`, the surface unmaps).
//! - A drag that was cancelled, or never started, leaves the card where it is and gives the
//!   hold `rest_ms` again.
//! - The service dropping the current shot (`Current(None)`) while shown leaves without a
//!   `Leave`: it already knows.

use sill_services::{ShotId, ShotLeave};

/// A hold timer's identity: a `HoldElapsed` with another token is stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HoldToken(pub u32);

/// Where the pointer is, as far as the hold cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pointer {
    Away,
    Over,
    /// Carrying the file out: the compositor has the pointer until the drag ends.
    Dragging,
}

/// Which motion carries a leaving card out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThumbExit {
    /// The card's own exit (quire's `ShotOut`), its row still listed.
    Card,
    /// A swipe: the row is unlisted and quire's stack flies it out along the swipe.
    Row,
}

/// The thumbnail's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThumbState {
    #[default]
    Hidden,
    Shown {
        id: ShotId,
        pointer: Pointer,
        token: HoldToken,
    },
    Leaving {
        id: ShotId,
        token: HoldToken,
        exit: ThumbExit,
    },
}

/// The machine's numbers, from the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThumbParams {
    /// `screenshot.thumbnail_hold_ms`.
    pub hold_ms: u32,
    /// What is left after the pointer leaves the card (the banners' `hover_min_remaining_ms`).
    pub rest_ms: u32,
}

/// How a drag out of the card ended (shell-host's `DragOutcome`, as the thumbnail reads it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DragEnd {
    /// Dropped and copied: the file stays where it was saved.
    Copied,
    /// Dropped and moved: the target has the file now.
    Moved,
    /// Released over nothing that took it.
    Cancelled,
    /// The drag never started.
    Refused,
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThumbIn {
    /// The service's current shot (on every change of `use_shots`).
    Current(Option<ShotId>),
    HoldElapsed(HoldToken),
    PointerEnter,
    PointerLeave,
    Click,
    /// A swipe right past quire's threshold.
    SwipedAway,
    /// A press on the card crossed quire's drag threshold: shot `id` is being carried out.
    DragStarted(ShotId),
    /// The drag of shot `id` ended.
    DragEnded(ShotId, DragEnd),
    /// The Delete action.
    TrashPressed,
    /// The exit motion finished.
    ExitSettled,
}

/// What to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThumbOut {
    /// Map the surface and play the entrance for `id`.
    Show(ShotId),
    /// Sleep this long with quire's timer, then feed `HoldElapsed(token)`.
    StartHold(HoldToken, u32),
    /// Tell the service (`ShotCommand::Leave`) and play the exit.
    Leave(ShotId, ShotLeave),
    /// Unmap the surface.
    Hide,
}

/// One step.
pub fn step(state: ThumbState, input: ThumbIn, params: ThumbParams) -> (ThumbState, Vec<ThumbOut>) {
    match state {
        ThumbState::Hidden => match input {
            ThumbIn::Current(Some(id)) => show(id, Pointer::Away, HoldToken(1), params),
            _ => (state, Vec::new()),
        },
        ThumbState::Shown { id, pointer, token } => shown(id, pointer, token, input, params),
        ThumbState::Leaving { id, token, .. } => match input {
            ThumbIn::ExitSettled => (ThumbState::Hidden, vec![ThumbOut::Hide]),
            ThumbIn::Current(Some(next)) if next != id => {
                show(next, Pointer::Away, next_token(token), params)
            }
            _ => (state, Vec::new()),
        },
    }
}

/// `id` shown with `token`: the entrance, and the hold unless the pointer is already on it.
fn show(
    id: ShotId,
    pointer: Pointer,
    token: HoldToken,
    params: ThumbParams,
) -> (ThumbState, Vec<ThumbOut>) {
    let state = ThumbState::Shown { id, pointer, token };
    let outs = match pointer {
        Pointer::Away => vec![
            ThumbOut::Show(id),
            ThumbOut::StartHold(token, params.hold_ms),
        ],
        Pointer::Over | Pointer::Dragging => vec![ThumbOut::Show(id)],
    };
    (state, outs)
}

/// Shown, the pointer away, and `rest_ms` more on the hold.
fn rested(id: ShotId, token: HoldToken, params: ThumbParams) -> (ThumbState, Vec<ThumbOut>) {
    let token = next_token(token);
    let state = ThumbState::Shown {
        id,
        pointer: Pointer::Away,
        token,
    };
    (state, vec![ThumbOut::StartHold(token, params.rest_ms)])
}

fn shown(
    id: ShotId,
    pointer: Pointer,
    token: HoldToken,
    input: ThumbIn,
    params: ThumbParams,
) -> (ThumbState, Vec<ThumbOut>) {
    let state = ThumbState::Shown { id, pointer, token };
    let leave = |why, exit| {
        (
            ThumbState::Leaving { id, token, exit },
            vec![ThumbOut::Leave(id, why)],
        )
    };
    let paused = |pointer| {
        let token = next_token(token);
        (ThumbState::Shown { id, pointer, token }, Vec::new())
    };
    match (input, pointer) {
        (ThumbIn::Current(Some(next)), _) if next != id => {
            show(next, pointer, next_token(token), params)
        }
        (ThumbIn::Current(None), _) => (
            ThumbState::Leaving {
                id,
                token,
                exit: ThumbExit::Card,
            },
            Vec::new(),
        ),
        (ThumbIn::HoldElapsed(elapsed), Pointer::Away) if elapsed == token => {
            leave(ShotLeave::Kept, ThumbExit::Card)
        }
        (ThumbIn::PointerEnter, Pointer::Away) => paused(Pointer::Over),
        (ThumbIn::PointerLeave, Pointer::Over) => rested(id, token, params),
        (ThumbIn::DragStarted(dragged), Pointer::Away | Pointer::Over) if dragged == id => {
            paused(Pointer::Dragging)
        }
        (ThumbIn::DragEnded(dragged, end), Pointer::Dragging) if dragged == id => match end {
            DragEnd::Copied => leave(ShotLeave::Dragged, ThumbExit::Card),
            DragEnd::Moved => leave(ShotLeave::Moved, ThumbExit::Card),
            DragEnd::Cancelled | DragEnd::Refused => rested(id, token, params),
        },
        // The drag of a shot since replaced: over, whatever became of it, so the hold resumes.
        (ThumbIn::DragEnded(..), Pointer::Dragging) => rested(id, token, params),
        (ThumbIn::Click, Pointer::Away | Pointer::Over) => {
            leave(ShotLeave::Opened, ThumbExit::Card)
        }
        (ThumbIn::SwipedAway, _) => leave(ShotLeave::Kept, ThumbExit::Row),
        (ThumbIn::TrashPressed, _) => leave(ShotLeave::Trashed, ThumbExit::Card),
        _ => (state, Vec::new()),
    }
}

fn next_token(token: HoldToken) -> HoldToken {
    HoldToken(token.0.wrapping_add(1))
}

#[cfg(test)]
#[path = "machine_table.rs"]
mod tests;
