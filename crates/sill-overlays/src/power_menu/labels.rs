//! The power menu's words, pure.

pub use sill_model::session::words::{consequence, prompt, title as action_label};

/// The alert's words when the session offers no action (no login manager reached, as in a bare
/// container): it says so, and Cancel is the only button.
pub const NONE_OFFERED_TITLE: &str = "No power actions available";
pub const NONE_OFFERED_MESSAGE: &str = "This session cannot shut down or restart the computer.";
