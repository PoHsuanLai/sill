//! The answer-carrying form of the stage: one sheet that is up or on its way out with an answer
//! (a [`ExitIn::Leave`] carries what happens next), handed to the caller once the exit has
//! settled. Pure; quire's `Sheet` plays the exit, and the caller feeds its `on_hidden` back as
//! [`ExitIn::Settled`].

use ds::prelude::*;

/// Whether a sheet is up, or leaving with the answer to carry out once it has gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetExit<A> {
    Open,
    Leaving(A),
}

/// What reaches the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitIn<A> {
    /// The sheet is dismissed or answered; `A` happens once it has left.
    Leave(A),
    /// The sheet's exit settled (quire's `on_hidden`).
    Settled,
    /// Shown again while leaving: it is up again and the answer is dropped.
    Reshown,
}

/// What the caller does after an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitOut<A> {
    Nothing,
    /// The sheet started to leave.
    Left,
    /// The exit has played: carry out the answer.
    Finished(A),
}

impl<A: Clone> SheetExit<A> {
    /// quire's `Sheet { shown }` for this phase.
    pub fn shown(&self) -> Shown {
        match self {
            SheetExit::Open => Shown::Visible,
            SheetExit::Leaving(_) => Shown::Hidden,
        }
    }

    /// The phase after `input`, and what the caller does. An answer given while leaving, and a
    /// settle while up, change nothing.
    pub fn advance(self, input: ExitIn<A>) -> (SheetExit<A>, ExitOut<A>) {
        match (self, input) {
            (SheetExit::Open, ExitIn::Leave(answer)) => (SheetExit::Leaving(answer), ExitOut::Left),
            (SheetExit::Leaving(answer), ExitIn::Settled) => (
                SheetExit::Leaving(answer.clone()),
                ExitOut::Finished(answer),
            ),
            (SheetExit::Leaving(_), ExitIn::Reshown) => (SheetExit::Open, ExitOut::Nothing),
            (exit, ExitIn::Leave(_) | ExitIn::Settled | ExitIn::Reshown) => {
                (exit, ExitOut::Nothing)
            }
        }
    }
}
