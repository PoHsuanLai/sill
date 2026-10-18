//! The routines of the helpers: what a helper does by itself when a time comes. The list, the
//! sheet where the person confirms a new one, and the transition ([`update`]) that moves them.
//! Pure: `sill-services` makes the calls [`RoutinesCommand`] asks for.

mod draft;
mod model;
mod msg;
#[cfg(test)]
mod tests;
mod update;
mod words;

pub use draft::{REPEATS, Repeat, RoutineDraft};
pub use model::{RoutineRemoval, RoutinesModel, Sheet};
pub use msg::{RoutinesCommand, RoutinesMsg};
pub use update::update;
pub use words::{
    MISSED_SKIPPED, NO_ROUTINES, last_run_line, remove_routine_question, repeat_label, state_label,
};
