//! What the surface shows, pure: the opening the link holds on this output, kept a moment past
//! its closing so quire's sheet can play its exit, and whether the surface is mapped. The next
//! opening replaces a leaving one at once (keyed by its serial, it enters again).

use ds::prelude::*;
use sill_ui_kit::links::ForceQuitOpening;

/// The surface's stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum Stage {
    /// Nothing drawn; the surface is unmapped.
    #[default]
    Empty,
    /// The panel is up.
    Up(ForceQuitOpening),
    /// The panel has closed; its sheet is playing its exit.
    Leaving(ForceQuitOpening),
}

/// What reaches the stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum StageIn {
    /// The opening on this output now (`None`: closed, or another output).
    Want(Option<ForceQuitOpening>),
    /// The sheet's exit settled (quire's `on_hidden`).
    Settled,
}

/// Whether the surface is mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Mapped {
    Yes,
    No,
}

/// `stage` after `input`.
pub(super) fn advance(stage: Stage, input: StageIn) -> Stage {
    match (stage, input) {
        (_, StageIn::Want(Some(opening))) => Stage::Up(opening),
        (Stage::Up(opening), StageIn::Want(None)) => Stage::Leaving(opening),
        (Stage::Leaving(_), StageIn::Settled) => Stage::Empty,
        (stage, StageIn::Want(None) | StageIn::Settled) => stage,
    }
}

impl Stage {
    /// Whether the surface is mapped: from the first frame of an opening to its exit's settle.
    pub(super) fn mapped(self) -> Mapped {
        match self {
            Stage::Empty => Mapped::No,
            Stage::Up(_) | Stage::Leaving(_) => Mapped::Yes,
        }
    }

    /// The opening drawn and quire's `shown` for it.
    pub(super) fn drawn(self) -> Option<(ForceQuitOpening, Shown)> {
        match self {
            Stage::Empty => None,
            Stage::Up(opening) => Some((opening, Shown::Visible)),
            Stage::Leaving(opening) => Some((opening, Shown::Hidden)),
        }
    }
}
