//! What the surface shows, pure: whether the voice service is asking, kept a moment past its
//! asking so quire's sheet can play its exit, and whether the surface is mapped.

use sill_model::voice::ConsentSheet;

/// The surface's stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stage {
    /// Nothing drawn; the surface is unmapped.
    #[default]
    Empty,
    /// The sheet is up.
    Up,
    /// The sheet has gone; it is playing its exit.
    Leaving,
}

/// What reaches the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StageIn {
    /// Whether the service asks, on this output (`Closed` also when the banners are elsewhere).
    Want(ConsentSheet),
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
        (_, StageIn::Want(ConsentSheet::Asking)) => Stage::Up,
        (Stage::Up, StageIn::Want(ConsentSheet::Closed)) => Stage::Leaving,
        (Stage::Leaving, StageIn::Settled) => Stage::Empty,
        (stage, StageIn::Want(ConsentSheet::Closed) | StageIn::Settled) => stage,
    }
}

impl Stage {
    /// Whether the surface is mapped: from the first frame of the sheet to its exit's settle.
    pub fn mapped(self) -> Mapped {
        match self {
            Stage::Empty => Mapped::No,
            Stage::Up | Stage::Leaving => Mapped::Yes,
        }
    }
}
