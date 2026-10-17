//! The helpers the person keeps, as the companion shows them: the list, the card being read or
//! filled in, what the helpers told the person, and the one transition ([`update`]) that moves
//! them. Pure: `sill-services` makes the calls [`HelpersCommand`] asks for and feeds back what
//! they came to, and the launcher draws [`HelpersModel`].

mod apps;
mod draft;
mod model;
mod models;
mod msg;
#[cfg(test)]
mod tests;
mod update;
mod words;

pub use apps::offered_apps;
pub use draft::{Draft, Reach};
pub use model::{HelpersModel, Open, Removal};
pub use models::{ModelGroup, ModelList, ModelOffer, model_groups};
pub use msg::{Asked, HelpersCommand, HelpersMsg};
pub use update::update;
pub use words::{
    ADD_MODEL, CEILINGS, GONE, LOOKING_FOR_MODELS, NO_MODELS, WAITING_BADGE, app_label, badge_of,
    ceiling_label, controls_of, job_answer, job_prompt, job_words, model_hint, model_line,
    remove_question,
};
