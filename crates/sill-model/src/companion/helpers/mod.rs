//! The helpers the person keeps, as the companion shows them: the list, the card being read or
//! filled in, what the helpers told the person, and the one transition ([`update`]) that moves
//! them. Pure: `sill-services` makes the calls [`HelpersCommand`] asks for and feeds back what
//! they came to, and the launcher draws [`HelpersModel`].

mod apps;
mod automatic;
mod draft;
mod model;
mod models;
mod msg;
mod routines;
mod sign_in;
#[cfg(test)]
mod tests;
mod update;
mod words;

pub use apps::offered_apps;
pub use automatic::{Automatic, PoolWritten};
pub use draft::{Draft, Reach};
pub use model::{HelpersModel, Open, Removal};
pub use models::{ModelGroup, ModelList, ModelOffer, model_groups};
pub use msg::{Asked, HelpersCommand, HelpersMsg};
pub use routines::{
    MISSED_SKIPPED, NO_ROUTINES, REPEATS, Repeat, RoutineDraft, RoutineRemoval, RoutinesCommand,
    RoutinesModel, RoutinesMsg, Sheet, last_run_line, remove_routine_question, repeat_label,
    state_label,
};
pub use sign_in::{
    SignInButton, kind_line, login_line, needs_sign_in_line, offers_sign_out, sign_in_buttons,
};
pub use update::update;
pub use words::{
    ADD_MODEL, AUTOMATIC_HELP, CEILINGS, GONE, LOOKING_FOR_MODELS, NO_MODELS, SHARED_WITH_HELPER,
    WAITING_BADGE, app_label, badge_of, ceiling_label, controls_of, job_answer, job_prompt,
    job_words, model_hint, model_line, remove_question,
};
