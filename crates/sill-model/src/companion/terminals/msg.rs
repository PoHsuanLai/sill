//! What tells the panel about sessions in terminals, and what it asks of the companion.

use companion_wire::{BroughtBack, TerminalSession};
use prov::SessionId;

/// What the panel is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalsMsg {
    /// The companion says the sessions in terminals changed: read them again.
    Changed,
    /// The sessions arrived.
    Listed(Vec<TerminalSession>),
    /// The companion could not be reached.
    Unreached,
    /// The person asks for a session back.
    BringBack(SessionId),
    /// The companion answered a bring back.
    Answered(SessionId, BroughtBack),
}

/// What the panel asks the companion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalsCommand {
    /// Read the sessions in terminals.
    List,
    /// Bring a session back.
    BringBack(SessionId),
}
