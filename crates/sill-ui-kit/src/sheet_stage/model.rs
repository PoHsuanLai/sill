//! The stage's state types.

use ds::prelude::*;
use shell_host::RegionSpec;

/// What a sheet's surface shows: nothing, the wanted value, or the value that has gone while its
/// sheet plays its exit. The next value replaces a leaving one at once (keyed by the caller, the
/// sheet enters again).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetStage<T> {
    /// Nothing drawn; the surface is unmapped.
    Empty,
    /// The value is up.
    Up(T),
    /// The value has gone; its sheet is playing its exit.
    Leaving(T),
}

/// What reaches the stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum StageIn<T> {
    /// The value on screen on this output now (`None`: none, or another output).
    Want(Option<T>),
    /// The sheet's exit settled (quire's `on_hidden`).
    Settled,
}

/// Whether the surface is mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mapped {
    Yes,
    No,
}

/// Whether the surface takes pointer input. A sheet takes it over its whole output only while it
/// is up: one playing its exit lets clicks through to what is under it at once, as a Mac sheet
/// does, so a click right after an answer reaches its window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pointer {
    Taken,
    Passed,
}

impl Pointer {
    /// The surface's input region for this.
    pub fn region(self) -> RegionSpec {
        match self {
            Pointer::Taken => RegionSpec::Whole,
            Pointer::Passed => RegionSpec::Empty,
        }
    }
}

impl<T> SheetStage<T> {
    /// Whether the surface is mapped: from the first frame of a value to its exit's settle.
    pub fn mapped(&self) -> Mapped {
        match self {
            SheetStage::Empty => Mapped::No,
            SheetStage::Up(_) | SheetStage::Leaving(_) => Mapped::Yes,
        }
    }

    /// Whether the surface takes pointer input: only while the value is up.
    pub fn pointer(&self) -> Pointer {
        match self {
            SheetStage::Up(_) => Pointer::Taken,
            SheetStage::Empty | SheetStage::Leaving(_) => Pointer::Passed,
        }
    }

    /// The value drawn and quire's `shown` for it.
    pub fn drawn(&self) -> Option<(&T, Shown)> {
        match self {
            SheetStage::Empty => None,
            SheetStage::Up(value) => Some((value, Shown::Visible)),
            SheetStage::Leaving(value) => Some((value, Shown::Hidden)),
        }
    }
}
