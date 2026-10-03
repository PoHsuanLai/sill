//! What the seat publishes: whether it is on the bus, the confirmations waiting for the person and
//! the terminal's standing grants.

use super::sheet::Sheets;
use super::terminal::Terminal;

/// Whether sill serves intentd's questions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum IntentsLink {
    /// Not asked to serve yet (the daemon's first `Serve` ends this); nothing touches a bus.
    #[default]
    Passive,
    /// `org.quire.Confirm1` and `org.quire.Shell` are ours.
    Serving,
    /// It could not serve; the sheet will never appear and intentd dismisses what it asks.
    Unavailable(LinkTrouble),
}

/// Why the seat is not serving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTrouble {
    /// No session bus to serve on.
    NoBus,
    /// Another process owns a name the seat needs; its text is the bus's.
    NameTaken(String),
    /// The bus refused something else; its message.
    Refused(String),
}

/// Everything the seat publishes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Intents {
    pub link: IntentsLink,
    /// The confirmations, the one on screen first.
    pub sheets: Sheets,
    /// The terminal's grants and the breaker.
    pub terminal: Terminal,
}
