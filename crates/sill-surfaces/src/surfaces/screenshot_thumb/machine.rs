//! The thumbnail's machine: pure, table-tested; the surface feeds it the service's current shot,
//! the pointer, the gestures and quire's timers, and carries out what it returns.
//!
//! - A new current shot shows (`Show`) and starts the hold (`StartHold` with a fresh token); a
//!   newer one while shown or leaving replaces it the same way.
//! - The pointer on the card pauses the hold (the token moves on, so the running timer's
//!   `HoldElapsed` is stale); leaving it starts `rest_ms` more.
//! - The hold running out with the pointer away, a swipe, a click, a drag out and Delete each
//!   leave: `Leave(id, why)` (the surface tells the service) and the state is `Leaving` while
//!   the exit plays; `ExitSettled` hides (`Hide`, the surface unmaps).
//! - The service dropping the current shot (`Current(None)`) while shown leaves without a
//!   `Leave`: it already knows.

use sill_services::{ShotId, ShotLeave};

/// A hold timer's identity: a `HoldElapsed` with another token is stale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct HoldToken(pub u32);

/// Where the pointer is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pointer {
    Away,
    Over,
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
    /// A drag out of the card was dropped somewhere.
    DraggedOut,
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
        ThumbState::Leaving { id, token } => match input {
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
        Pointer::Over => vec![ThumbOut::Show(id)],
    };
    (state, outs)
}

fn shown(
    id: ShotId,
    pointer: Pointer,
    token: HoldToken,
    input: ThumbIn,
    params: ThumbParams,
) -> (ThumbState, Vec<ThumbOut>) {
    let state = ThumbState::Shown { id, pointer, token };
    let leave = |why| {
        (
            ThumbState::Leaving { id, token },
            vec![ThumbOut::Leave(id, why)],
        )
    };
    match (input, pointer) {
        (ThumbIn::Current(Some(next)), _) if next != id => {
            show(next, pointer, next_token(token), params)
        }
        (ThumbIn::Current(None), _) => (ThumbState::Leaving { id, token }, Vec::new()),
        (ThumbIn::HoldElapsed(elapsed), Pointer::Away) if elapsed == token => {
            leave(ShotLeave::Kept)
        }
        (ThumbIn::PointerEnter, Pointer::Away) => (
            ThumbState::Shown {
                id,
                pointer: Pointer::Over,
                token: next_token(token),
            },
            Vec::new(),
        ),
        (ThumbIn::PointerLeave, Pointer::Over) => {
            let token = next_token(token);
            (
                ThumbState::Shown {
                    id,
                    pointer: Pointer::Away,
                    token,
                },
                vec![ThumbOut::StartHold(token, params.rest_ms)],
            )
        }
        (ThumbIn::Click, _) => leave(ShotLeave::Opened),
        (ThumbIn::SwipedAway, _) => leave(ShotLeave::Kept),
        (ThumbIn::DraggedOut, _) => leave(ShotLeave::Dragged),
        (ThumbIn::TrashPressed, _) => leave(ShotLeave::Trashed),
        _ => (state, Vec::new()),
    }
}

fn next_token(token: HoldToken) -> HoldToken {
    HoldToken(token.0.wrapping_add(1))
}

#[cfg(test)]
mod tests {
    use sill_services::{ShotId, ShotLeave};

    use super::{HoldToken, Pointer, ThumbIn, ThumbOut, ThumbParams, ThumbState, step};

    const P: ThumbParams = ThumbParams {
        hold_ms: 5200,
        rest_ms: 1500,
    };
    const ONE: ShotId = ShotId(1);
    const TWO: ShotId = ShotId(2);

    fn shown(id: ShotId, pointer: Pointer, token: u32) -> ThumbState {
        ThumbState::Shown {
            id,
            pointer,
            token: HoldToken(token),
        }
    }

    #[test]
    fn the_thumbnail_follows_its_table() {
        use Pointer::{Away, Over};
        use ThumbIn::*;
        let leaving = |id, token| ThumbState::Leaving {
            id,
            token: HoldToken(token),
        };
        let cases: Vec<(ThumbState, ThumbIn, ThumbState, Vec<ThumbOut>)> = vec![
            (
                ThumbState::Hidden,
                Current(Some(ONE)),
                shown(ONE, Away, 1),
                vec![ThumbOut::Show(ONE), ThumbOut::StartHold(HoldToken(1), 5200)],
            ),
            (
                ThumbState::Hidden,
                Current(None),
                ThumbState::Hidden,
                vec![],
            ),
            (ThumbState::Hidden, Click, ThumbState::Hidden, vec![]),
            (
                shown(ONE, Away, 1),
                Current(Some(ONE)),
                shown(ONE, Away, 1),
                vec![],
            ),
            (
                shown(ONE, Away, 1),
                Current(Some(TWO)),
                shown(TWO, Away, 2),
                vec![ThumbOut::Show(TWO), ThumbOut::StartHold(HoldToken(2), 5200)],
            ),
            (
                shown(ONE, Away, 1),
                PointerEnter,
                shown(ONE, Over, 2),
                vec![],
            ),
            (
                shown(ONE, Over, 2),
                HoldElapsed(HoldToken(1)),
                shown(ONE, Over, 2),
                vec![],
            ),
            (
                shown(ONE, Over, 2),
                PointerLeave,
                shown(ONE, Away, 3),
                vec![ThumbOut::StartHold(HoldToken(3), 1500)],
            ),
            (
                shown(ONE, Away, 3),
                HoldElapsed(HoldToken(3)),
                leaving(ONE, 3),
                vec![ThumbOut::Leave(ONE, ShotLeave::Kept)],
            ),
            (
                shown(ONE, Over, 2),
                Click,
                leaving(ONE, 2),
                vec![ThumbOut::Leave(ONE, ShotLeave::Opened)],
            ),
            (
                shown(ONE, Over, 2),
                SwipedAway,
                leaving(ONE, 2),
                vec![ThumbOut::Leave(ONE, ShotLeave::Kept)],
            ),
            (
                shown(ONE, Over, 2),
                DraggedOut,
                leaving(ONE, 2),
                vec![ThumbOut::Leave(ONE, ShotLeave::Dragged)],
            ),
            (
                shown(ONE, Over, 2),
                TrashPressed,
                leaving(ONE, 2),
                vec![ThumbOut::Leave(ONE, ShotLeave::Trashed)],
            ),
            (shown(ONE, Away, 1), Current(None), leaving(ONE, 1), vec![]),
            (
                leaving(ONE, 2),
                ExitSettled,
                ThumbState::Hidden,
                vec![ThumbOut::Hide],
            ),
            (leaving(ONE, 2), Click, leaving(ONE, 2), vec![]),
            (
                leaving(ONE, 2),
                Current(Some(TWO)),
                shown(TWO, Away, 3),
                vec![ThumbOut::Show(TWO), ThumbOut::StartHold(HoldToken(3), 5200)],
            ),
        ];
        for (from, input, want, outs) in cases {
            assert_eq!(step(from, input, P), (want, outs), "{from:?} + {input:?}");
        }
    }
}
