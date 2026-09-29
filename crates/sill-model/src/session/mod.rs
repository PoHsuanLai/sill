//! The session's power actions: what logind offers, as the power menu draws it.

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use command::{SessionCommand, SessionEvent};
pub use model::{Offer, Session, SessionAction};
pub use step::step;
