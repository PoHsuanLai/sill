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
//! - The keyboard is never taken on appear. The shared ask (`FocusAsked`, from `sill focus-latest`)
//!   gives the card the keyboard (`TakeKeyboard`) and pauses the hold like the pointer does; the
//!   keys then act as the card's buttons do (Enter and Space open it in quire's card; Delete and
//!   Backspace are `TrashPressed`; Escape is `Dismissed`: the hold's own leave, the file kept). Every
//!   way out of `Shown` while the keyboard is held gives it back (`ReleaseKeyboard`), and the
//!   compositor taking it away (`FocusLost`) gives the hold `rest_ms` again.
//! - A drag that was cancelled, or never started, leaves the card where it is and gives the
//!   hold `rest_ms` again.
//! - The service dropping the current shot (`Current(None)`) while shown leaves without a
//!   `Leave`: it already knows.

use sill_model::screenshots::{ShotId, ShotLeave};

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

/// Whether the card holds the keyboard: only when the person asked for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyFocus {
    Away,
    Held,
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
        keys: KeyFocus,
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
    /// The service's current shot (on every change of `use_service::<ScreenshotsService>`).
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
    /// The Delete action, or the Delete or Backspace key.
    TrashPressed,
    /// The Escape key: dismissed like the hold running out, the file kept.
    Dismissed,
    /// `sill focus-latest`: the person wants the keyboard on the card.
    FocusAsked,
    /// The compositor took the keyboard away.
    FocusLost,
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
    /// Ask the compositor for the keyboard and put the focus on the card.
    TakeKeyboard,
    /// Give the keyboard back to what had it.
    ReleaseKeyboard,
}

/// One step.
pub fn step(state: ThumbState, input: ThumbIn, params: ThumbParams) -> (ThumbState, Vec<ThumbOut>) {
    match state {
        ThumbState::Hidden => match input {
            ThumbIn::Current(Some(id)) => show(id, Pointer::Away, HoldToken(1), params),
            _ => (state, Vec::new()),
        },
        ThumbState::Shown {
            id,
            pointer,
            keys,
            token,
        } => shown(
            Card {
                id,
                pointer,
                keys,
                token,
            },
            input,
            params,
        ),
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
    let state = ThumbState::Shown {
        id,
        pointer,
        keys: KeyFocus::Away,
        token,
    };
    let outs = match pointer {
        Pointer::Away => vec![
            ThumbOut::Show(id),
            ThumbOut::StartHold(token, params.hold_ms),
        ],
        Pointer::Over | Pointer::Dragging => vec![ThumbOut::Show(id)],
    };
    (state, outs)
}

/// The fields of `Shown`, taken apart to be changed together.
#[derive(Debug, Clone, Copy)]
struct Card {
    id: ShotId,
    pointer: Pointer,
    keys: KeyFocus,
    token: HoldToken,
}

impl Card {
    fn state(self) -> ThumbState {
        ThumbState::Shown {
            id: self.id,
            pointer: self.pointer,
            keys: self.keys,
            token: self.token,
        }
    }

    fn leaving(self, exit: ThumbExit) -> ThumbState {
        ThumbState::Leaving {
            id: self.id,
            token: self.token,
            exit,
        }
    }

    /// Leaving because of `why`: the service is told, and the keyboard given back if held.
    fn leave(self, why: ShotLeave, exit: ThumbExit) -> (ThumbState, Vec<ThumbOut>) {
        let outs = self.releasing([ThumbOut::Leave(self.id, why)]);
        (self.leaving(exit), outs)
    }

    /// `outs`, with the keyboard given back first if the card holds it.
    fn releasing(self, outs: impl IntoIterator<Item = ThumbOut>) -> Vec<ThumbOut> {
        let release = (self.keys == KeyFocus::Held).then_some(ThumbOut::ReleaseKeyboard);
        release.into_iter().chain(outs).collect()
    }

    /// The pointer and keyboard as given, the running timer stale; `rest_ms` more on the hold
    /// only when neither the pointer nor the keyboard is on the card.
    fn resumed(
        self,
        pointer: Pointer,
        keys: KeyFocus,
        params: ThumbParams,
    ) -> (ThumbState, Vec<ThumbOut>) {
        let card = Card {
            pointer,
            keys,
            token: next_token(self.token),
            ..self
        };
        let outs = match (pointer, keys) {
            (Pointer::Away, KeyFocus::Away) => {
                vec![ThumbOut::StartHold(card.token, params.rest_ms)]
            }
            _ => Vec::new(),
        };
        (card.state(), outs)
    }
}

fn shown(card: Card, input: ThumbIn, params: ThumbParams) -> (ThumbState, Vec<ThumbOut>) {
    let Card {
        id,
        pointer,
        keys,
        token,
    } = card;
    let resumed = |pointer, keys| card.resumed(pointer, keys, params);
    match (input, pointer) {
        (ThumbIn::Current(Some(next)), _) if next != id => {
            let (state, outs) = show(next, pointer, next_token(token), params);
            (state, card.releasing(outs))
        }
        (ThumbIn::Current(None), _) => (card.leaving(ThumbExit::Card), card.releasing([])),
        (ThumbIn::HoldElapsed(elapsed), Pointer::Away) if elapsed == token => {
            card.leave(ShotLeave::Kept, ThumbExit::Card)
        }
        (ThumbIn::PointerEnter, Pointer::Away) => resumed(Pointer::Over, keys),
        (ThumbIn::PointerLeave, Pointer::Over) => resumed(Pointer::Away, keys),
        (ThumbIn::DragStarted(dragged), Pointer::Away | Pointer::Over) if dragged == id => {
            resumed(Pointer::Dragging, keys)
        }
        (ThumbIn::DragEnded(dragged, end), Pointer::Dragging) if dragged == id => match end {
            DragEnd::Copied => card.leave(ShotLeave::Dragged, ThumbExit::Card),
            DragEnd::Moved => card.leave(ShotLeave::Moved, ThumbExit::Card),
            DragEnd::Cancelled | DragEnd::Refused => resumed(Pointer::Away, keys),
        },
        // The drag of a shot since replaced: over, whatever became of it, so the hold resumes.
        (ThumbIn::DragEnded(..), Pointer::Dragging) => resumed(Pointer::Away, keys),
        (ThumbIn::Click, Pointer::Away | Pointer::Over) => {
            card.leave(ShotLeave::Opened, ThumbExit::Card)
        }
        (ThumbIn::SwipedAway, _) => card.leave(ShotLeave::Kept, ThumbExit::Row),
        (ThumbIn::TrashPressed, _) => card.leave(ShotLeave::Trashed, ThumbExit::Card),
        (ThumbIn::Dismissed, Pointer::Away | Pointer::Over) => {
            card.leave(ShotLeave::Kept, ThumbExit::Card)
        }
        (ThumbIn::FocusAsked, _) => {
            let (state, _) = resumed(pointer, KeyFocus::Held);
            (state, vec![ThumbOut::TakeKeyboard])
        }
        (ThumbIn::FocusLost, _) if keys == KeyFocus::Held => {
            let (state, outs) = resumed(pointer, KeyFocus::Away);
            let outs = [ThumbOut::ReleaseKeyboard].into_iter().chain(outs);
            (state, outs.collect())
        }
        _ => (card.state(), Vec::new()),
    }
}

fn next_token(token: HoldToken) -> HoldToken {
    HoldToken(token.0.wrapping_add(1))
}

#[cfg(test)]
#[path = "step_table.rs"]
mod tests;
