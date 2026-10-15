//! The stage every sheet that plays an exit before it closes shares: what the surface wants on
//! screen, kept a moment past its going so quire's `Sheet` can animate out, and whether the
//! surface is mapped. [`step::advance`] is the pure machine, [`io::use_sheet_stage`] follows a
//! wanted value onto it (and so onto the surface's mapping) and [`io::settle`] is what the
//! sheet's `on_hidden` calls once the exit has played.

mod io;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use io::{settle, use_sheet_stage};
pub use model::{Mapped, SheetStage};
