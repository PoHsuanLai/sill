//! The queue and the words around it (data only, and the one rule about what an offer allows).

use docket_core::{ConfirmAnswer, ConfirmOffer, ConfirmRequest, NeverOffer};

/// One `Confirm` call, as the service counts them. It names the request object the answer goes
/// to; a `ConfirmId` is the router's and can repeat if the router is wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Serial(pub u64);

/// A question and the call it came in on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    pub serial: Serial,
    pub request: ConfirmRequest,
}

/// The confirmations waiting for the person, the one on screen first (first in, first out).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sheets {
    pub queue: Vec<Pending>,
}

impl Sheets {
    /// The question on screen.
    pub fn current(&self) -> Option<&Pending> {
        self.queue.first()
    }

    /// How many more wait behind it.
    pub fn waiting(&self) -> usize {
        self.queue.len().saturating_sub(1)
    }
}

/// What the person chose on the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SheetChoice {
    /// Yes, this once.
    AllowOnce,
    /// Yes, from now on (offered only as `OnceOrAlways`).
    AllowAlways,
    /// Yes, and the same action may run from the terminal until logout.
    AllowFromTerminal,
    /// No, and not again: this call is refused and a "never" is kept for the action (offered only
    /// when the request says so).
    Never,
    /// No.
    Deny,
}

/// What goes back on the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The answer: `Request.Response(0, answer)`.
    Answer(ConfirmAnswer),
    /// The router withdrew the question: `Request.Response(1, "")`.
    Withdrawn,
}

/// Whether the sheet may be answered with `choice` when it asked `request`: "always" only when
/// the router offered it, "from the terminal" only when it offered that, "never" only when the
/// request's limits offer it, and a plain yes or no whatever was offered.
pub fn permits(request: &ConfirmRequest, choice: SheetChoice) -> bool {
    match choice {
        SheetChoice::Deny | SheetChoice::AllowOnce => true,
        SheetChoice::AllowAlways => request.offer == ConfirmOffer::OnceOrAlways,
        SheetChoice::AllowFromTerminal => request.offer == ConfirmOffer::OnceOrFromTerminal,
        SheetChoice::Never => request.limits.never == NeverOffer::Offered,
    }
}
