//! What "Never" and a held limit add to the sheet: the answer the person gives, what the sheet
//! says of the limit behind it, and Undo taking that line away.

use docket_core::{
    ConfirmAnswer, ConfirmEnd, ConfirmOffer, ConfirmRequest, NeverOffer, SheetLimits,
};
use docket_permissions_core::{LimitLevel, LimitNote, StandingGrantId, UndoOffer};
use prov::{Confidentiality, ConfirmReceipt, InputProof, UnixSeconds};

use super::{
    Pending, Reply, Serial, SheetChoice, SheetEffect, SheetEvent, Sheets, sheet_view, step,
};
use crate::intents::tests::{id, request};

const AT: UnixSeconds = UnixSeconds(1_700_000_000);

fn entry(hex: &str) -> StandingGrantId {
    StandingGrantId::parse(&format!("sg-{hex}")).unwrap_or_else(|e| panic!("{hex}: {e}"))
}

fn note(hex: &str, undo: UndoOffer) -> LimitNote {
    LimitNote {
        id: entry(hex),
        level: LimitLevel::AlwaysAsk,
        why: "You said: \u{201c}don't touch Work\u{201d}".to_owned(),
        undo,
    }
}

fn limited(id_text: &str, never: NeverOffer, held: Option<LimitNote>) -> ConfirmRequest {
    request(id_text, ConfirmOffer::OnceOnly, 120).limiting(SheetLimits { never, held })
}

fn queue_of(requests: Vec<ConfirmRequest>) -> Sheets {
    let queue = requests
        .into_iter()
        .zip(1..)
        .map(|(request, serial)| Pending {
            serial: Serial(serial),
            request,
        })
        .collect();
    Sheets { queue }
}

#[test]
fn never_answers_with_the_sheets_receipt_only_when_the_request_offers_it() {
    let receipt = ConfirmReceipt {
        id: id("c-1"),
        input: InputProof::SheetFallback,
        at: AT,
        covers: Confidentiality::Secret,
    };
    let answered = SheetEvent::Answered {
        id: id("c-1"),
        choice: SheetChoice::Never,
        at: AT,
    };
    // The request, the queue after the choice, and what is sent back.
    let cases = [
        (
            "offered",
            NeverOffer::Offered,
            0,
            vec![SheetEffect::Respond(
                Serial(1),
                Reply::Answer(ConfirmAnswer::Never { receipt }),
            )],
        ),
        ("withheld", NeverOffer::Withheld, 1, Vec::new()),
    ];
    for (name, never, left, want) in cases {
        let sheets = queue_of(vec![limited("c-1", never, None)]);
        let (sheets, effects) = step(sheets, answered.clone());
        assert_eq!(sheets.queue.len(), left, "{name}: the queue");
        assert_eq!(effects, want, "{name}: the reply");
    }
}

#[test]
fn a_refusal_that_is_not_never_keeps_its_old_answer() {
    let sheets = queue_of(vec![limited("c-1", NeverOffer::Offered, None)]);
    let deny = SheetEvent::Answered {
        id: id("c-1"),
        choice: SheetChoice::Deny,
        at: AT,
    };
    let (_, effects) = step(sheets, deny);
    assert_eq!(
        effects,
        vec![SheetEffect::Respond(
            Serial(1),
            Reply::Answer(ConfirmAnswer::Ended(ConfirmEnd::Refused))
        )]
    );
}

#[test]
fn the_view_says_whether_never_is_offered_and_which_limit_held() {
    let cases = [
        ("no limits", NeverOffer::Withheld, None),
        ("never only", NeverOffer::Offered, None),
        (
            "a held limit",
            NeverOffer::Offered,
            Some(note("00000000000000a1", UndoOffer::Offered)),
        ),
        (
            "an administrator's limit",
            NeverOffer::Withheld,
            Some(note("00000000000000a2", UndoOffer::Withheld)),
        ),
    ];
    for (name, never, held) in cases {
        let sheets = queue_of(vec![limited("c-1", never, held.clone())]);
        let view = sheet_view(&sheets).unwrap_or_else(|| panic!("{name}: a view"));
        assert_eq!(view.never, never, "{name}: never");
        let seen = view
            .held
            .as_ref()
            .map(|h| (h.id.clone(), h.why.clone(), h.undo));
        let want = held.map(|n| (n.id, n.why, n.undo));
        assert_eq!(seen, want, "{name}: held");
    }
}

#[test]
fn undoing_a_limit_takes_it_off_every_question_that_named_it_and_nothing_else() {
    let (mine, other) = (entry("00000000000000b1"), entry("00000000000000b2"));
    let sheets = queue_of(vec![
        limited(
            "c-1",
            NeverOffer::Offered,
            Some(note("00000000000000b1", UndoOffer::Offered)),
        ),
        limited(
            "c-2",
            NeverOffer::Offered,
            Some(note("00000000000000b2", UndoOffer::Offered)),
        ),
        limited("c-3", NeverOffer::Offered, None),
    ]);
    let held = |sheets: &Sheets| -> Vec<Option<StandingGrantId>> {
        let notes = sheets.queue.iter().map(|p| p.request.limits.held.as_ref());
        notes.map(|held| held.map(|n| n.id.clone())).collect()
    };
    assert_eq!(
        held(&sheets),
        [Some(mine.clone()), Some(other.clone()), None]
    );
    let (after, effects) = step(sheets, SheetEvent::LimitUndone(mine));
    assert_eq!(held(&after), [None, Some(other), None]);
    assert_eq!(after.queue.len(), 3, "the questions stay for an answer");
    assert!(effects.is_empty(), "nothing is sent: the entry is gone");
}

#[test]
fn never_is_a_no_that_asks_for_no_grant_to_be_read() {
    use crate::intents::{Intents, IntentsEffect, IntentsEvent, step as seat_step};
    let state = Intents {
        sheets: queue_of(vec![limited("c-1", NeverOffer::Offered, None)]),
        ..Intents::default()
    };
    let never = IntentsEvent::Sheet(SheetEvent::Answered {
        id: id("c-1"),
        choice: SheetChoice::Never,
        at: AT,
    });
    let (state, effects) = seat_step(state, never);
    assert!(state.sheets.queue.is_empty());
    let [IntentsEffect::Sheet(SheetEffect::Respond(Serial(1), Reply::Answer(answer)))] =
        effects.as_slice()
    else {
        panic!("one reply and nothing else: {effects:?}");
    };
    assert!(matches!(answer, ConfirmAnswer::Never { .. }), "{answer:?}");
}
