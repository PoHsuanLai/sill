//! What the surface shows, pure: the request the capture service wants on this output, kept a
//! moment past its going so quire's sheet can play its exit, and whether the surface is mapped.
//! The next request replaces a leaving one at once (keyed by its id, it enters again).

use ds::prelude::*;
use sill_model::capture_pick::Asking;

/// The surface's stage.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Stage {
    /// Nothing drawn; the surface is unmapped.
    #[default]
    Empty,
    /// The request is up.
    Up(Asking),
    /// The request has gone; its sheet is playing its exit.
    Leaving(Asking),
}

/// What reaches the stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageIn {
    /// The request on screen on this output now (`None`: none, or another output).
    Want(Option<Asking>),
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
        (_, StageIn::Want(Some(asking))) => Stage::Up(asking),
        (Stage::Up(asking), StageIn::Want(None)) => Stage::Leaving(asking),
        (Stage::Leaving(_), StageIn::Settled) => Stage::Empty,
        (stage, StageIn::Want(None) | StageIn::Settled) => stage,
    }
}

impl Stage {
    /// Whether the surface is mapped: from the first frame of a request to its exit's settle.
    pub fn mapped(&self) -> Mapped {
        match self {
            Stage::Empty => Mapped::No,
            Stage::Up(_) | Stage::Leaving(_) => Mapped::Yes,
        }
    }

    /// The request drawn and quire's `shown` for it.
    pub fn drawn(&self) -> Option<(&Asking, Shown)> {
        match self {
            Stage::Empty => None,
            Stage::Up(asking) => Some((asking, Shown::Visible)),
            Stage::Leaving(asking) => Some((asking, Shown::Hidden)),
        }
    }
}
