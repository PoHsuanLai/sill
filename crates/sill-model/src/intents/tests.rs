//! The seat as a whole: the queue and the terminal moved through [`step`], and the fixtures the
//! parts' tests share.

use docket_core::{
    Anchor, AskReason, ConfirmAnswer, ConfirmDetail, ConfirmId, ConfirmOffer, ConfirmRequest,
    Gesture, LabelText, Seconds, TaintNote,
};
use porter_core::Count;
use prov::{Actor, AppName, Effect, SpaceId, UnixSeconds};

use super::sheet::{Reply, Serial, SheetChoice, SheetEffect, SheetEvent};
use super::terminal::{Breaker, TerminalEvent};
use super::{Intents, IntentsEffect, IntentsEvent, IntentsLink, LinkTrouble, step};

pub(crate) fn app(name: &str) -> AppName {
    AppName::parse(name).unwrap_or_else(|e| panic!("{name}: {e}"))
}

pub(crate) fn id(text: &str) -> ConfirmId {
    ConfirmId::parse(text).unwrap_or_else(|e| panic!("{text}: {e}"))
}

/// A terminal call's request `id` offering `offer`, expiring after `expires` seconds.
pub(crate) fn request(id_text: &str, offer: ConfirmOffer, expires: u32) -> ConfirmRequest {
    ConfirmRequest {
        id: id(id_text),
        space: SpaceId::parse("work").unwrap_or_else(|e| panic!("{e}")),
        actor: Actor::Cli,
        app: app("org.quire.Mail"),
        action: LabelText::parse("Archive a thread").unwrap_or_else(|e| panic!("{e}")),
        effect: Effect::UndoableWrite,
        count: Count(1),
        detail: ConfirmDetail::Plain,
        lines: Vec::new(),
        why: vec![AskReason::FromTerminal],
        taint: sheet_taint(),
        offer,
        gesture: Gesture::Press,
        anchor: Anchor::Centre,
        expires: Seconds(expires),
    }
}

fn sheet_taint() -> TaintNote {
    TaintNote::Clean
}

pub(crate) fn asked(serial: u64, request: ConfirmRequest) -> IntentsEvent {
    IntentsEvent::Sheet(SheetEvent::Asked(Serial(serial), Box::new(request)))
}

pub(crate) fn answered(id_text: &str, choice: SheetChoice) -> IntentsEvent {
    IntentsEvent::Sheet(SheetEvent::Answered {
        id: id(id_text),
        choice,
        at: UnixSeconds(1_700_000_000),
    })
}

#[test]
fn a_link_that_stops_serving_drops_the_questions_nobody_waits_for() {
    let serving = step(Intents::default(), IntentsEvent::Link(IntentsLink::Serving)).0;
    let (queued, _) = step(
        serving,
        asked(1, request("c-1", ConfirmOffer::OnceOnly, 120)),
    );
    assert_eq!(queued.sheets.queue.len(), 1);
    for link in [
        IntentsLink::Passive,
        IntentsLink::Unavailable(LinkTrouble::NoBus),
    ] {
        let (after, effects) = step(queued.clone(), IntentsEvent::Link(link.clone()));
        assert_eq!(after.link, link);
        assert!(after.sheets.queue.is_empty(), "{link:?}");
        assert!(effects.is_empty(), "no one is left to answer: {link:?}");
    }
    let (kept, _) = step(queued.clone(), IntentsEvent::Link(IntentsLink::Serving));
    assert_eq!(kept.sheets, queued.sheets, "serving again keeps the queue");
}

#[test]
fn allowing_from_the_terminal_asks_for_the_grants_to_be_read_again() {
    let (state, _) = step(
        Intents::default(),
        asked(1, request("c-1", ConfirmOffer::OnceOrFromTerminal, 120)),
    );
    let (state, effects) = step(state, answered("c-1", SheetChoice::AllowFromTerminal));
    assert!(state.sheets.queue.is_empty());
    assert_eq!(effects.len(), 2);
    assert!(matches!(
        &effects[0],
        IntentsEffect::Sheet(SheetEffect::Respond(
            Serial(1),
            Reply::Answer(ConfirmAnswer::AllowedFromTerminal { receipt })
        )) if receipt.id == id("c-1")
    ));
    assert_eq!(effects[1], IntentsEffect::ReadGrantsSoon);
    // A plain yes does not.
    let (state, _) = step(
        state,
        asked(2, request("c-2", ConfirmOffer::OnceOrFromTerminal, 120)),
    );
    let (_, effects) = step(state, answered("c-2", SheetChoice::AllowOnce));
    assert!(!effects.contains(&IntentsEffect::ReadGrantsSoon));
}

#[test]
fn the_terminal_moves_by_its_own_events() {
    let grant = docket_core::ActionRef {
        app: app("org.quire.Mail"),
        name: prov::ActionName::parse("mail.thread.archive").unwrap_or_else(|e| panic!("{e}")),
    };
    let (state, effects) = step(
        Intents::default(),
        IntentsEvent::Terminal(TerminalEvent::Loaded(vec![grant.clone()])),
    );
    assert_eq!(state.terminal.grants, vec![grant]);
    assert!(effects.is_empty());
    let (state, _) = step(
        state,
        IntentsEvent::Terminal(TerminalEvent::Breaker(Breaker::Tripped)),
    );
    assert_eq!(state.terminal.breaker, Breaker::Tripped);
}

#[test]
fn a_receipt_never_claims_hardware() {
    let (state, _) = step(
        Intents::default(),
        asked(1, request("c-1", ConfirmOffer::OnceOnly, 120)),
    );
    let (_, effects) = step(state, answered("c-1", SheetChoice::AllowOnce));
    let IntentsEffect::Sheet(SheetEffect::Respond(
        _,
        Reply::Answer(ConfirmAnswer::Allowed { receipt, .. }),
    )) = &effects[0]
    else {
        panic!("{effects:?}");
    };
    assert_eq!(receipt.input, prov::InputProof::SheetFallback);
}
