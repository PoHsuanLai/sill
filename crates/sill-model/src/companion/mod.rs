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
mod emphasis;
mod helpers;
#[cfg(test)]
mod memory_line_tests;
mod model;
mod model_picker;
#[cfg(test)]
mod model_picker_tests;
#[cfg(test)]
mod openers_tests;
mod presence;
mod program_labels;
mod progress;
#[cfg(test)]
mod progress_tests;
#[cfg(test)]
mod refusal_tests;
mod run_state;
mod step;
mod summon;
mod tap;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod waiting_tests;
mod words;

pub use card::{card_call, cards_of, openers_of, promote_card, unresolved_handles};
pub use command::{
    AskInput, CompanionCommand, CompanionEffect, CompanionEvent, CompanionParams, FocusInput,
};
pub use emphasis::{Emphasis, Span, plain_of, spans};
pub use helpers::{
    ADD_MODEL, AUTOMATIC_HELP, Asked, Automatic, CEILINGS, Draft, GONE, HelpersCommand,
    HelpersModel, HelpersMsg, LOOKING_FOR_MODELS, MISSED_SKIPPED, ModelGroup, ModelList,
    ModelOffer, NO_MODELS, NO_ROUTINES, Open, PoolWritten, REPEATS, Reach, Removal, Repeat,
    RoutineDraft, RoutineRemoval, RoutinesCommand, RoutinesModel, RoutinesMsg, SHARED_WITH_HELPER,
    Sheet, SignInButton, WAITING_BADGE, app_label, badge_of, ceiling_label, controls_of,
    job_answer, job_prompt, job_words, kind_line, last_run_line, login_line, model_groups,
    model_hint, model_line, needs_sign_in_line, offered_apps, offers_sign_out, remove_question,
    remove_routine_question, repeat_label, sign_in_buttons, state_label, update as update_helpers,
};
pub use model::{
    AnswerFacts, Busy, Companion, CompanionLink, Lookup, LookupPhase, Opener, RunFacts,
};
pub use presence::presence_of;
pub use program_labels::ProgramLabels;
pub use progress::{ProgressShown, preview_progress};
pub use run_state::{RunState, going_runs, run_controls, run_state_of};
pub use step::{step, wake};
pub use summon::{
    Fallback, FocusFacts, InField, LauncherFacts, LauncherOrigin, Serves, Summon, SummonCx,
    SummonIn, SummonOut, SummonParams,
};
pub use tap::{Tap, TapIn, TapOut, TapParams};
pub use words::{SETTLE_FAILED, activity_line, answer_line, body_lines, memory_line, waiting_line};
