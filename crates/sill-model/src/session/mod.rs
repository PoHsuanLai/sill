//! The session's power actions: what logind offers, and what each is called (`words`).

mod command;
mod model;
mod step;
pub mod words;
#[cfg(test)]
mod tests;

pub use command::{SessionCommand, SessionEvent};
pub use model::{Offer, Session, SessionAction};
pub use step::step;
