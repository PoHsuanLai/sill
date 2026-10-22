//! A turn that met no model says to choose one, and opens nothing by itself.

use std::collections::BTreeMap;

use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, FooterWire, RefusalWire};
use docket_core::{ContextKeep, Keep, Reveal};
use prov::TaskId;

use super::no_model::{CHOOSE_MODEL_LABEL, NO_MODEL_LINE, lacks_model};
use super::*;

pub(super) fn task() -> TaskId {
    TaskId::parse("t-1").unwrap_or_else(|e| panic!("{e}"))
}

pub(super) fn wire(phase: AnswerPhase, body: AnswerBody) -> AnswerWire {
    AnswerWire {
        task: task(),
        phase,
        body,
        progress: Vec::new(),
        footer: FooterWire {
            mode: docket_core::Mode::default(),
            served: Vec::new(),
            sources: Vec::new(),
            keep: ContextKeep {
                query: Keep::Dropped,
                results: Keep::Dropped,
                selection: Keep::Dropped,
                window: Keep::Dropped,
            },
        },
    }
}

fn no_model() -> AnswerWire {
    wire(
        AnswerPhase::Failed,
        AnswerBody::Refused(RefusalWire::NoModel),
    )
}

fn text() -> AnswerWire {
    wire(
        AnswerPhase::Done,
        AnswerBody::Text {
            lines: vec![Reveal::Plain("Two found.".to_owned())],
        },
    )
}

pub(super) fn held(prompt: &str, wire: AnswerWire) -> Companion {
    let facts = AnswerFacts {
        prompt: prompt.to_owned(),
        wire,
        busy: Busy::Idle,
        session: None,
        routing: Vec::new(),
        shown: BTreeMap::new(),
    };
    Companion {
        answers: BTreeMap::from([(task(), facts)]),
        ..Companion::default()
    }
}

#[test]
fn only_a_refusal_for_want_of_a_model_lacks_one() {
    let cases = [
        ("refused for want of a model", no_model(), true),
        ("answered", text(), false),
        (
            "refused for another reason",
            wire(
                AnswerPhase::Failed,
                AnswerBody::Refused(RefusalWire::Failed("x".to_owned())),
            ),
            false,
        ),
    ];
    for (name, answer, want) in cases {
        assert_eq!(lacks_model(&answer.body), want, "{name}");
    }
}

#[test]
fn the_turn_that_met_no_model_says_to_choose_one() {
    let cases = [("failed", AnswerPhase::Failed), ("done", AnswerPhase::Done)];
    for (name, phase) in cases {
        let state = held(
            "find the receipts",
            wire(phase, AnswerBody::Refused(RefusalWire::NoModel)),
        );
        let (title, subtitle) = answer_line(&state.answers[&task()]);
        assert_eq!(
            (title.as_str(), subtitle.as_str()),
            ("find the receipts", NO_MODEL_LINE),
            "{name}"
        );
    }
    assert_eq!(
        (NO_MODEL_LINE, CHOOSE_MODEL_LABEL),
        (
            "Choose a model to use the assistant.",
            "Choose Model\u{2026}"
        )
    );
}

#[test]
fn a_refusal_opens_nothing_by_itself() {
    let events = [
        CompanionEvent::Asked {
            task: task(),
            prompt: "find the receipts".to_owned(),
            session: prov::SessionId::parse("s-1").unwrap_or_else(|e| panic!("{e}")),
        },
        CompanionEvent::Answer(Box::new(no_model())),
        CompanionEvent::Answer(Box::new(no_model())),
    ];
    let mut state = Companion::default();
    for event in events {
        let (next, effects) = step(
            state,
            event,
            ds_core::time::stamp::Stamp(0),
            &CompanionParams::default(),
        );
        assert_eq!(effects, Vec::new());
        state = next;
    }
}

#[test]
fn no_model_is_the_companions_word_for_it() {
    let text = serde_json::to_string(&RefusalWire::NoModel).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(text, r#"{"kind":"no_model"}"#);
    let back: RefusalWire =
        serde_json::from_str(r#"{"kind":"no_model"}"#).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, RefusalWire::NoModel);
}
