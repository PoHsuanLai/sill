//! The queue's transition.

use docket_core::{ConfirmAnswer, ConfirmEnd, ConfirmId, ConfirmRequest, GrantScope, Seconds};
use prov::{Confidentiality, ConfirmReceipt, InputProof, UnixSeconds};

use super::model::{Pending, Reply, Serial, SheetChoice, Sheets, permits};

/// What happens to the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetEvent {
    /// A `Confirm` call arrived.
    Asked(Serial, Box<ConfirmRequest>),
    /// The person chose on the sheet showing `id`, at `at`.
    Answered {
        id: ConfirmId,
        choice: SheetChoice,
        at: UnixSeconds,
    },
    /// The router withdrew `id` (`Cancel`).
    Withdrawn(ConfirmId),
    /// A question's time ran out.
    Expired(Serial),
    /// The asker left the bus or the seat stopped serving: nobody waits, so nothing is answered.
    AskerGone,
}

/// What the service must do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetEffect {
    /// Answer the call `Serial` came in on.
    Respond(Serial, Reply),
    /// Start `Serial`'s clock: after `after` it expires.
    Expire { serial: Serial, after: Seconds },
}

/// `sheets` after `event`.
pub fn step(sheets: Sheets, event: SheetEvent) -> (Sheets, Vec<SheetEffect>) {
    match event {
        SheetEvent::Asked(serial, request) => asked(sheets, serial, *request),
        SheetEvent::Answered { id, choice, at } => answered(sheets, &id, choice, at),
        SheetEvent::Withdrawn(id) => ended(sheets, |p| p.request.id == id, Reply::Withdrawn),
        SheetEvent::Expired(serial) => ended(
            sheets,
            |p| p.serial == serial,
            Reply::Answer(ConfirmAnswer::Ended(ConfirmEnd::Expired)),
        ),
        SheetEvent::AskerGone => (Sheets::default(), Vec::new()),
    }
}

/// A question joins the queue and starts its clock; a second one under an id already waiting is
/// the router's mistake, and is dismissed rather than shown twice.
fn asked(sheets: Sheets, serial: Serial, request: ConfirmRequest) -> (Sheets, Vec<SheetEffect>) {
    if sheets.queue.iter().any(|p| p.request.id == request.id) {
        let dismissed = Reply::Answer(ConfirmAnswer::Ended(ConfirmEnd::Dismissed));
        return (sheets, vec![SheetEffect::Respond(serial, dismissed)]);
    }
    let expire = SheetEffect::Expire {
        serial,
        after: request.expires,
    };
    let mut queue = sheets.queue;
    queue.push(Pending { serial, request });
    (Sheets { queue }, vec![expire])
}

/// The person's choice answers the question under `id` when its offer allows the choice; a
/// choice it does not allow changes nothing.
fn answered(
    sheets: Sheets,
    id: &ConfirmId,
    choice: SheetChoice,
    at: UnixSeconds,
) -> (Sheets, Vec<SheetEffect>) {
    let Some(place) = sheets.queue.iter().position(|p| &p.request.id == id) else {
        return (sheets, Vec::new());
    };
    if !permits(sheets.queue[place].request.offer, choice) {
        return (sheets, Vec::new());
    }
    let mut queue = sheets.queue;
    let gone = queue.remove(place);
    let answer = answer_of(&gone.request, choice, at);
    (
        Sheets { queue },
        vec![SheetEffect::Respond(gone.serial, Reply::Answer(answer))],
    )
}

/// The answer on the wire. The receipt says the person answered on the sheet drawn by a stock
/// compositor's client (phase A): `SheetFallback`, never `HardwareSeat`, which only the
/// compositor can attest. It covers nothing a sheet's action opens (`Secret`).
fn answer_of(request: &ConfirmRequest, choice: SheetChoice, at: UnixSeconds) -> ConfirmAnswer {
    let receipt = ConfirmReceipt {
        id: request.id.clone(),
        input: InputProof::SheetFallback,
        at,
        covers: Confidentiality::Secret,
    };
    match choice {
        SheetChoice::AllowOnce => ConfirmAnswer::Allowed {
            scope: GrantScope::Once,
            receipt,
        },
        SheetChoice::AllowAlways => ConfirmAnswer::Allowed {
            scope: GrantScope::Always,
            receipt,
        },
        SheetChoice::AllowFromTerminal => ConfirmAnswer::AllowedFromTerminal { receipt },
        SheetChoice::Deny => ConfirmAnswer::Ended(ConfirmEnd::Refused),
    }
}

/// The first question `which` picks leaves the queue with `reply`.
fn ended(
    sheets: Sheets,
    which: impl Fn(&Pending) -> bool,
    reply: Reply,
) -> (Sheets, Vec<SheetEffect>) {
    let Some(place) = sheets.queue.iter().position(which) else {
        return (sheets, Vec::new());
    };
    let mut queue = sheets.queue;
    let gone = queue.remove(place);
    (
        Sheets { queue },
        vec![SheetEffect::Respond(gone.serial, reply)],
    )
}
