//! The stage's state types.

use ds::prelude::*;

use crate::surface::Pointer;

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

impl<T> SheetStage<T> {
    /// Whether the surface is mapped: from the first frame of a value to its exit's settle.
    pub fn mapped(&self) -> Mapped {
        match self {
            SheetStage::Empty => Mapped::No,
            SheetStage::Up(_) | SheetStage::Leaving(_) => Mapped::Yes,
        }
    }

    /// Whether the surface takes the pointer: only while the value is up, so a sheet playing its
    /// exit lets a click through to what is under it at once, as a Mac sheet does.
    pub fn pointer(&self) -> Pointer {
        match self {
            SheetStage::Up(_) => Pointer::Takes,
            SheetStage::Empty | SheetStage::Leaving(_) => Pointer::PassesThrough,
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
