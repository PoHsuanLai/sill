//! The session's power actions: what logind offers, and what each is called (`words`).

mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;
pub mod words;

pub use command::{SessionCommand, SessionEvent};
pub use model::{Offer, Session, SessionAction};
pub use step::step;
