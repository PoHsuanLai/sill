//! Another of the person's computers asking to borrow this computer's models, as the shell tells
//! the person: a notification per asking computer, "Let <name> use this computer's models?", with
//! Allow and Deny. A press answers inferd; the notification is withdrawn when the question is
//! answered, here or elsewhere (Settings).

mod command;
mod model;
mod notice;
mod posted;
mod step;
#[cfg(test)]
mod tests;

pub use command::{GuestAsksCommand, GuestAsksEffect, GuestAsksEvent};
pub use model::{Ask, Computer, GuestAsksState, Verdict};
pub use notice::{NOTICE_ENTRY, Notice, choice_key, choice_of, notice_text, shown_for};
pub use posted::Posted;
pub use step::step;
