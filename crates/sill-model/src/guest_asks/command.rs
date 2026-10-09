//! What can happen to the guest-asks service, what it can be told, and what it must then do.

use super::model::{Ask, Computer, Verdict};

/// What the service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestAsksCommand {
    /// Read the questions waiting again.
    Reread,
}

/// One thing that happened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestAsksEvent {
    /// A computer began asking.
    Asked(Ask),
    /// The questions waiting now, as inferd lists them; anything else is answered or gone.
    Waiting(Vec<Ask>),
    /// The person pressed a button.
    Chosen(Computer, Verdict),
}

/// What the loop does as a result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuestAsksEffect {
    /// Show the notification for the question, in place of one already shown.
    Tell(Ask),
    /// Take the computer's notification back.
    Withdraw(Computer),
    /// Give inferd the person's answer.
    Answer(Computer, Verdict),
}
