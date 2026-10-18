//! The coding-agent sessions the person started in a terminal: the list the companion keeps of
//! them, the words a row wears in the panel, and the one transition ([`update`]) that takes the
//! list and the person's Bring Back. Pure: `sill-services` makes the calls [`TerminalsCommand`]
//! asks for and feeds back what they came to.

mod model;
mod msg;
#[cfg(test)]
mod tests;
mod update;
mod words;

pub use model::{RefusedBringBack, TerminalsModel};
pub use msg::{TerminalsCommand, TerminalsMsg};
pub use update::update;
pub use words::{BRING_BACK_LABEL, NO_BRING_BACK, session_subtitle, session_title};
