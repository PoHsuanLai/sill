//! What the seat publishes: whether it is on the bus, the confirmations waiting for the person and
//! the terminal's standing grants.

use super::sheet::Sheets;
use super::terminal::Terminal;
use crate::serve::ServeLink;

/// Whether sill serves intentd's questions: `org.quire.Confirm1` and `org.quire.Shell` are ours.
/// When it could not, the sheet will never appear and intentd dismisses what it asks.
pub type IntentsLink = ServeLink;

/// Everything the seat publishes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Intents {
    pub link: IntentsLink,
    /// The confirmations, the one on screen first.
    pub sheets: Sheets,
    /// The terminal's grants and the breaker.
    pub terminal: Terminal,
}
