//! The companion: where a summon goes, what the companion has answered, the computer-use runs,
//! the undo journal, and the words each of them wears in the launcher.
//!
//! The pure half of the companion service. [`Tap`] and [`Summon`] are timed [`Machine`]s on
//! quire's clock (the service owns and steps them); [`presence_of`] derives the one presence
//! the orb and the glow show; [`run_state_of`] and [`run_controls`] read a run; [`step`] keeps
//! the published [`Companion`]. `sill-services` does the bus.
//!
//! [`Machine`]: ds_core::machine::Machine

mod card;
mod command;
mod model;
#[cfg(test)]
mod openers_tests;
mod presence;
mod run_state;
mod step;
mod summon;
mod tap;
#[cfg(test)]
mod tests;
mod words;

pub use card::{card_call, cards_of, openers_of, promote_card, unresolved_handles};
pub use command::{
    AskInput, CompanionCommand, CompanionEffect, CompanionEvent, CompanionParams, FocusInput,
};
pub use model::{
    AnswerFacts, Busy, Companion, CompanionLink, Lookup, LookupPhase, Opener, RunFacts,
};
pub use presence::presence_of;
pub use run_state::{RunState, going_runs, run_controls, run_state_of};
pub use step::{step, wake};
pub use summon::{
    Fallback, FocusFacts, InField, LauncherFacts, LauncherOrigin, Serves, Summon, SummonCx,
    SummonIn, SummonOut, SummonParams,
};
pub use tap::{Tap, TapIn, TapOut, TapParams};
pub use words::{activity_line, answer_line, memory_line};
