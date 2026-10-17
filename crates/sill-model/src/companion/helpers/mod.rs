//! The helpers the person keeps, as the companion shows them: the list, the card being read or
//! filled in, what the helpers told the person, and the one transition ([`update`]) that moves
//! them. Pure: `sill-services` makes the calls [`HelpersCommand`] asks for and feeds back what
//! they came to, and the launcher draws [`HelpersModel`].

mod apps;
mod draft;
mod model;
mod msg;
#[cfg(test)]
mod tests;
mod update;
mod words;

pub use apps::offered_apps;
pub use draft::{Draft, Reach, SaveFault, reach_of_lines};
pub use model::{HelpersModel, Open, OpenCard, Removal, Told};
pub use msg::{Asked, HelpersCommand, HelpersMsg};
pub use update::update;
pub use words::{
    CEILINGS, CHOOSE_MODEL, GONE, NO_JOB_WORDS, UNREACHED, WAITING_BADGE, badge_of, ceiling_label,
    controls_of, job_prompt, job_state_label, job_words, model_hint, model_line, remove_question,
};
