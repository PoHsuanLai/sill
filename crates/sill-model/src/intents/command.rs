//! What tells the seat things, what happens in it, and what that asks of the outside.

use docket_core::{ActionRef, ConfirmId};
use serde::{Deserialize, Serialize};

use super::IntentsLink;
use super::sheet::{SheetChoice, SheetEffect, SheetEvent};
use super::terminal::TerminalEvent;

/// What the daemon and the surfaces tell the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentsCommand {
    /// Take the bus names and serve. The service is passive until the first one.
    Serve,
    /// The person answered the confirmation `id` on screen.
    Answer { id: ConfirmId, choice: SheetChoice },
    /// Withdraw one standing terminal grant.
    RevokeGrant(ActionRef),
    /// Let the terminal's sessions the breaker paused take calls again.
    ResumeTerminal,
    /// Read the standing grants again (the control center opened).
    RefreshGrants,
}

/// The part of [`IntentsCommand`] that crosses the IPC wire: what `sill intents` asks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum IntentsAsk {
    /// Withdraw one standing terminal grant.
    Revoke(ActionRef),
    /// Let the terminal's paused sessions take calls again.
    Resume,
}

impl From<IntentsAsk> for IntentsCommand {
    fn from(ask: IntentsAsk) -> Self {
        match ask {
            IntentsAsk::Revoke(grant) => IntentsCommand::RevokeGrant(grant),
            IntentsAsk::Resume => IntentsCommand::ResumeTerminal,
        }
    }
}

/// What happens in the seat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentsEvent {
    /// The seat's place on the bus changed.
    Link(IntentsLink),
    /// Something happened to the confirmation queue.
    Sheet(SheetEvent),
    /// Something happened to the terminal's grants or the breaker.
    Terminal(TerminalEvent),
}

/// What the service must do outside the pure state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntentsEffect {
    /// A sheet's own effect: answer a call, or start a question's expiry.
    Sheet(SheetEffect),
    /// A standing terminal grant was just given: read the grants again shortly.
    ReadGrantsSoon,
}
