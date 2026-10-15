//! What a limit adds to the confirmation sheet, on the same harness as `confirm_sheet`: "Never"
//! beside the allow choices when the request offers it (not the default button, no key of its
//! own, answering `Never`); the line a held "ask first" entry puts on the sheet, with Undo, which
//! sends the undo and leaves the question open; and no "Always Allow" where the request does
//! not offer it.

use std::time::Duration;

use ds::prelude::*;
use ds_harness::{Driver, Input, Query};
use sill_model::intents::sheet::{Pending, Serial, SheetChoice};
use sill_model::intents::{IntentsCommand, StandingGrantId};

use crate::confirm_sheet::{ALLOW, DENY, OFFER, answer, armed, arming, ask, click, sent, wire};

const NEVER: &str = "#confirm-never";
const UNDO: &str = "#confirm-undo";
const HELD_LINE: &str = ".sill-confirm-held-why";

const NEVER_OFFERED: &str = r#"{"never":"offered"}"#;
const NEVER_WITHHELD: &str = r#"{"never":"withheld"}"#;
const HELD_UNDOABLE: &str = r#"{"never":"offered","held":{"id":"sg-00000000000000a1",
    "level":"always_ask","why":"You said: “don't touch Work”","undo":"offered"}}"#;
const HELD_LOCKED: &str = r#"{"never":"withheld","held":{"id":"sg-00000000000000a2",
    "level":"always_ask","why":"Your administrator requires asking first.","undo":"withheld"}}"#;

const WHY_SAID: &str = "You said: \u{201c}don't touch Work\u{201d}";

/// The question `id` as the router would send it with these `limits` (their wire form).
fn limited(id: &str, offer: &str, limits: &str) -> Pending {
    let mut value: serde_json::Value =
        serde_json::from_str(&wire(id, offer, "press", "undoable_write"))
            .unwrap_or_else(|e| panic!("{e}"));
    value["limits"] = serde_json::from_str(limits).unwrap_or_else(|e| panic!("{e}"));
    Pending {
        serial: Serial(1),
        request: serde_json::from_value(value).unwrap_or_else(|e| panic!("{e}")),
    }
}

fn entry(hex: &str) -> StandingGrantId {
    StandingGrantId::parse(&format!("sg-{hex}")).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn never_is_a_button_only_when_the_request_offers_it() {
    // The limits, the offer, whether Never shows, and whether Always Allow does beside it.
    let cases = [
        ("offered", NEVER_OFFERED, "once_only", 1, false),
        ("withheld", NEVER_WITHHELD, "once_only", 0, false),
        (
            "offered beside always",
            NEVER_OFFERED,
            "once_or_always",
            1,
            true,
        ),
        (
            "withheld beside always",
            NEVER_WITHHELD,
            "once_or_always",
            0,
            true,
        ),
    ];
    for (name, limits, offer, never, always) in cases {
        let running = armed(vec![limited("c-1", offer, limits)]);
        let h = &running.harness;
        assert_eq!(h.count(NEVER), never, "{name}: Never");
        assert_eq!(
            h.html().contains("Always Allow"),
            always,
            "{name}: Always Allow"
        );
        assert_eq!(
            h.text_of(ALLOW).as_deref(),
            Some("Allow"),
            "{name}: Allow stays"
        );
        assert_eq!(
            h.text_of(DENY).as_deref(),
            Some("Don't Allow"),
            "{name}: Don't Allow stays"
        );
    }
    let running = armed(vec![limited("c-1", "once_only", NEVER_OFFERED)]);
    assert_eq!(running.harness.text_of(NEVER).as_deref(), Some("Never"));
}

#[test]
fn never_answers_never_and_is_not_the_default_button() {
    let mut running = armed(vec![limited("c-1", "once_only", NEVER_OFFERED)]);
    let h = &running.harness;
    assert_eq!(h.attr(NEVER, "data-answers"), None, "no key is Never's");
    assert!(
        h.attr(ALLOW, "data-answers").is_some(),
        "Allow is still the default"
    );
    assert!(
        h.attr(DENY, "data-answers").is_some(),
        "Don't Allow still answers Escape"
    );
    click(&mut running, NEVER);
    assert_eq!(sent(&mut running), vec![answer("c-1", SheetChoice::Never)]);
}

#[test]
fn return_still_allows_with_never_on_the_sheet() {
    let mut running = armed(vec![limited("c-1", "once_only", NEVER_OFFERED)]);
    running.harness.send(Input::key(ShortcutKey::Enter));
    running.harness.advance(Duration::from_millis(40));
    assert_eq!(
        sent(&mut running),
        vec![answer("c-1", SheetChoice::AllowOnce)]
    );
}

#[test]
fn never_is_disabled_while_the_sheet_arms_and_answers_nothing() {
    let mut running = arming(vec![limited("c-1", "once_only", NEVER_OFFERED)]);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(
        running.harness.attr(NEVER, "data-availability").as_deref(),
        Some("disabled")
    );
    click(&mut running, NEVER);
    assert!(sent(&mut running).is_empty());
}

#[test]
fn a_held_limit_is_a_line_with_undo_and_the_sheet_offers_no_always() {
    let running = armed(vec![limited("c-1", "once_only", HELD_UNDOABLE)]);
    let h = &running.harness;
    assert_eq!(h.text_of(HELD_LINE).as_deref(), Some(WHY_SAID));
    assert_eq!(h.text_of(UNDO).as_deref(), Some("Undo"));
    assert_eq!(h.count(OFFER), 0, "nothing else is offered over a limit");
    assert!(!h.html().contains("Always Allow"));
    let plain = armed(vec![limited("c-1", "once_only", NEVER_OFFERED)]);
    assert_eq!(
        plain.harness.count(".sill-confirm-held"),
        0,
        "no limit, no line"
    );
    assert_eq!(plain.harness.count(UNDO), 0);
}

#[test]
fn undo_sends_the_undo_and_leaves_the_question_open() {
    let mut running = armed(vec![limited("c-1", "once_only", HELD_UNDOABLE)]);
    click(&mut running, UNDO);
    assert_eq!(
        sent(&mut running),
        vec![IntentsCommand::UndoLimit(entry("00000000000000a1"))],
        "the undo, and no answer"
    );
    assert_eq!(running.harness.count(".sill-confirm"), 1, "the sheet stays");
    // The service has said the entry is gone: the question loses its line and keeps its place.
    ask(
        &mut running,
        vec![limited("c-1", "once_only", NEVER_OFFERED)],
    );
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(running.harness.count(".sill-confirm-held"), 0);
    assert_eq!(
        running.harness.attr(".sill-confirm", "data-arm").as_deref(),
        Some("armed"),
        "the same question is not armed again"
    );
}

#[test]
fn undo_waits_for_the_arming_like_every_answer() {
    let mut running = arming(vec![limited("c-1", "once_only", HELD_UNDOABLE)]);
    running.harness.advance(Duration::from_millis(100));
    assert_eq!(
        running.harness.attr(UNDO, "data-availability").as_deref(),
        Some("disabled")
    );
    click(&mut running, UNDO);
    assert!(sent(&mut running).is_empty());
}

#[test]
fn an_entry_that_is_not_the_persons_has_its_reason_and_no_undo() {
    let running = armed(vec![limited("c-1", "once_only", HELD_LOCKED)]);
    assert_eq!(
        running.harness.text_of(HELD_LINE).as_deref(),
        Some("Your administrator requires asking first.")
    );
    assert_eq!(running.harness.count(UNDO), 0);
    assert_eq!(
        running.harness.count(NEVER),
        0,
        "the request did not offer it"
    );
}
