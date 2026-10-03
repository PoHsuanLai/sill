//! What the surface shows, pure: the question the intents service wants on this output, kept a
//! moment past its going so quire's sheet can play its exit, and whether the surface is mapped.
//! The next question replaces a leaving one at once (keyed by its id, it enters again).

use ds::prelude::*;
use sill_model::intents::sheet::SheetView;

/// The surface's stage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Stage {
    /// Nothing drawn; the surface is unmapped.
    #[default]
    Empty,
    /// The question is up.
    Up(SheetView),
    /// The question has gone; its sheet is playing its exit.
    Leaving(SheetView),
}

/// What reaches the stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageIn {
    /// The question on screen on this output now (`None`: none, or another output).
    Want(Option<SheetView>),
    /// The sheet's exit settled (quire's `on_hidden`).
    Settled,
}

/// Whether the surface is mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mapped {
    Yes,
    No,
}

/// `stage` after `input`.
pub fn advance(stage: Stage, input: StageIn) -> Stage {
    match (stage, input) {
        // The same question again (its `waiting` count moved) keeps what is on screen.
        (Stage::Up(now), StageIn::Want(Some(next))) if now.id == next.id => Stage::Up(next),
        (_, StageIn::Want(Some(view))) => Stage::Up(view),
        (Stage::Up(view), StageIn::Want(None)) => Stage::Leaving(view),
        (Stage::Leaving(_), StageIn::Settled) => Stage::Empty,
        (stage, StageIn::Want(None) | StageIn::Settled) => stage,
    }
}

impl Stage {
    /// Whether the surface is mapped: from the first frame of a question to its exit's settle.
    pub fn mapped(&self) -> Mapped {
        match self {
            Stage::Empty => Mapped::No,
            Stage::Up(_) | Stage::Leaving(_) => Mapped::Yes,
        }
    }

    /// The view drawn and quire's `shown` for it.
    pub fn drawn(&self) -> Option<(&SheetView, Shown)> {
        match self {
            Stage::Empty => None,
            Stage::Up(view) => Some((view, Shown::Visible)),
            Stage::Leaving(view) => Some((view, Shown::Hidden)),
        }
    }
}
