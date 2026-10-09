//! The state types (data only).

/// A computer by its Tailscale node id: opaque here, and the only thing a signal, a notification's
/// button and the answer all say it by.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Computer(String);

impl Computer {
    /// The node id inferd gave.
    pub fn new(node: impl Into<String>) -> Computer {
        Computer(node.into())
    }

    /// The node id as inferd spells it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The person's answer to a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Allow,
    Deny,
}

/// A computer waiting for the person's yes or no.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    pub node: Computer,
    /// What the person calls the computer.
    pub name: String,
}

/// The computers asking now, in the order they were told.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuestAsksState {
    pub asks: Vec<Ask>,
}
