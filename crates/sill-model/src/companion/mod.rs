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
mod mode;
#[cfg(test)]
mod mode_tests;
mod model;
mod no_model;
#[cfg(test)]
mod no_model_tests;
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
mod sight;
mod step;
mod summon;
mod tap;
mod terminals;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod waiting_tests;
mod words;

pub use card::{card_call, cards_of, openers_of, promote_card, unresolved_handles};
pub use command::{
    AskInput, CompanionCommand, CompanionEffect, CompanionEvent, CompanionParams, FocusInput,
};
/// The conversation and mode words of the router, for the crates that name them without docket.
pub use docket_permissions_core::Mode;
pub use emphasis::{Emphasis, Span, plain_of, spans};
pub use helpers::{
    ADD_MODEL, AUTOMATIC_HELP, Asked, Automatic, BRING_BACK, CEILINGS, Draft, GONE, HandoverShown,
    HelpersCommand, HelpersModel, HelpersMsg, IN_TERMINAL, LOOKING_FOR_MODELS, MISSED_SKIPPED,
    ModelGroup, ModelList, ModelOffer, NO_MODELS, NO_ROUTINES, OPEN_IN_TERMINAL, Open,
    PROPOSAL_ACCEPT, PROPOSAL_DECLINE, PROPOSAL_TITLE, PROPOSAL_WHY_LEAD, REPEATS, Reach, Removal,
    Repeat, RoutineDraft, RoutineRemoval, RoutinesCommand, RoutinesModel, RoutinesMsg,
    SHARED_WITH_HELPER, Sheet, SignInButton, Takes, WAITING_BADGE, app_label, badge_of,
    ceiling_label, controls_of, handover_of, job_answer, job_prompt, job_words, kind_line,
    last_run_line, login_line, model_groups, model_hint, model_line, needs_sign_in_line,
    offered_apps, offers_sign_out, proposal_what, proposal_when, proposal_why, remove_question,
    remove_routine_question, repeat_label, sign_in_buttons, state_label, update as update_helpers,
};
pub use mode::{
    ConversationMode, FROM_NEXT_STEP, LOCKED_HELP, ModeMsg, ModeNote, ModeView, SWITCH_NAME,
    TIP_NAME, mode_label, mode_view, other_mode, update as update_mode,
};
pub use model::{AnswerFacts, Busy, Companion, Lookup, LookupPhase, Opener, RunFacts};
pub use no_model::{CHOOSE_MODEL_LABEL, NO_MODEL_LINE, lacks_model};
pub use presence::presence_of;
pub use program_labels::ProgramLabels;
pub use progress::{ProgressShown, preview_progress};
pub use prov::SessionId;
pub use run_state::{RunState, going_runs, run_controls, run_state_of};
pub use sight::{NoteText, SightState};
pub use step::{step, wake};
pub use summon::{
    Fallback, FocusFacts, InField, LauncherOrigin, Summon, SummonCx, SummonIn, SummonOut,
    SummonParams,
};
pub use tap::{Tap, TapIn, TapOut, TapParams};
pub use terminals::{
    BRING_BACK_LABEL, NO_BRING_BACK, RefusedBringBack, TerminalsCommand, TerminalsModel,
    TerminalsMsg, session_subtitle, session_title, update as update_terminals,
};
pub use words::{SETTLE_FAILED, activity_line, answer_line, body_lines, memory_line, waiting_line};
