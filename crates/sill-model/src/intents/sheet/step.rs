//! The queue's transition.

use docket_core::{
    ConfirmAnswer, ConfirmEnd, ConfirmId, ConfirmRequest, GrantScope, Seconds, StandingGrantId,
};
use prov::{Confidentiality, ConfirmReceipt, InputProof, UnixSeconds};

use super::brief::BRIEF_MAX_BYTES;
use super::brief_wire::{allowed_once, brief_of};
use super::model::{Pending, Reply, SHEET_QUEUE_MAX, Serial, SheetChoice, Sheets, permits};

/// What happens to the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetEvent {
    /// A `Confirm` call arrived.
    Asked(Serial, Box<ConfirmRequest>),
    /// The person pressed "Use this brief" on the brief sheet `id`, with `text` as it stood in the
    /// field.
    BriefUsed {
        id: ConfirmId,
        text: String,
        at: UnixSeconds,
    },
    /// The person chose on the sheet showing `id`, at `at`.
    Answered {
        id: ConfirmId,
        choice: SheetChoice,
        at: UnixSeconds,
    },
    /// The limit entry that made a sheet ask was undone: no sheet names it as its reason any more.
    LimitUndone(StandingGrantId),
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
        SheetEvent::BriefUsed { id, text, at } => brief_used(sheets, &id, text, at),
        SheetEvent::LimitUndone(id) => (unheld(sheets, &id), Vec::new()),
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
/// the router's mistake, and is dismissed rather than shown twice, and so is one that arrives
/// when [`SHEET_QUEUE_MAX`] already wait.
fn asked(sheets: Sheets, serial: Serial, request: ConfirmRequest) -> (Sheets, Vec<SheetEffect>) {
    let waiting = sheets.queue.len() >= SHEET_QUEUE_MAX;
    if waiting || sheets.queue.iter().any(|p| p.request.id == request.id) {
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
    if !permits(&sheets.queue[place].request, choice) {
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

/// The brief sheet `id` is answered with `text`, the words as the person left them. Only a brief
/// sheet takes it, and only with some words that fit the bound; anything else changes nothing,
/// so the call ends as an unanswered one does.
fn brief_used(
    sheets: Sheets,
    id: &ConfirmId,
    text: String,
    at: UnixSeconds,
) -> (Sheets, Vec<SheetEffect>) {
    let Some(place) = sheets.queue.iter().position(|p| &p.request.id == id) else {
        return (sheets, Vec::new());
    };
    let usable = !text.trim().is_empty() && text.len() <= BRIEF_MAX_BYTES;
    if !usable || brief_of(&sheets.queue[place].request).is_none() {
        return (sheets, Vec::new());
    }
    let mut queue = sheets.queue;
    let gone = queue.remove(place);
    let answer = allowed_once(receipt_of(&gone.request, at), Some(text));
    (
        Sheets { queue },
        vec![SheetEffect::Respond(gone.serial, Reply::Answer(answer))],
    )
}

/// The answer on the wire. The receipt says the person answered on the sheet drawn by a stock
/// compositor's client (phase A): `SheetFallback`, never `HardwareSeat`, which only the
/// compositor can attest. It covers nothing a sheet's action opens (`Secret`).
fn answer_of(request: &ConfirmRequest, choice: SheetChoice, at: UnixSeconds) -> ConfirmAnswer {
    let receipt = receipt_of(request, at);
    match choice {
        SheetChoice::AllowOnce => ConfirmAnswer::Allowed {
            scope: GrantScope::Once,
            receipt,
            written: None,
        },
        SheetChoice::AllowAlways => ConfirmAnswer::Allowed {
            scope: GrantScope::Always,
            receipt,
            written: None,
        },
        SheetChoice::AllowFromTerminal => ConfirmAnswer::AllowedFromTerminal { receipt },
        SheetChoice::Never => ConfirmAnswer::Never { receipt },
        SheetChoice::Deny => ConfirmAnswer::Ended(ConfirmEnd::Refused),
    }
}

/// The proof of an answer given on this sheet.
fn receipt_of(request: &ConfirmRequest, at: UnixSeconds) -> ConfirmReceipt {
    ConfirmReceipt {
        id: request.id.clone(),
        input: InputProof::SheetFallback,
        at,
        covers: Confidentiality::Secret,
    }
}

/// `sheets` with the limit `id` taken off every question that named it as its reason: the entry
/// is gone, so the line and its Undo no longer describe anything. The questions stay; the call
/// still waits for the person's answer.
fn unheld(sheets: Sheets, id: &StandingGrantId) -> Sheets {
    let queue = sheets
        .queue
        .into_iter()
        .map(|mut pending| {
            let named = pending.request.limits.held.as_ref().map(|note| &note.id);
            if named == Some(id) {
                pending.request.limits.held = None;
            }
            pending
        })
        .collect();
    Sheets { queue }
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
