//! A turn that met no model opens the picker once and says nothing itself.

use std::collections::BTreeMap;

use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, FooterWire, RefusalWire};
use docket_core::{ContextKeep, Keep, Reveal};
use ds_core::time::stamp::Stamp;
use prov::{SessionId, TaskId};

use super::model_picker::{Picker, opens, picker_of};
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

fn asked() -> CompanionEvent {
    CompanionEvent::Asked {
        task: task(),
        prompt: "find the receipts".to_owned(),
        session: SessionId::parse("s-1").unwrap_or_else(|e| panic!("{e}")),
    }
}

fn answer(wire: AnswerWire) -> CompanionEvent {
    CompanionEvent::Answer(Box::new(wire))
}

/// Feed `events` in order; the effects each one asked for.
fn effects_of(events: Vec<CompanionEvent>) -> Vec<Vec<CompanionEffect>> {
    let mut state = Companion::default();
    let mut all = Vec::new();
    for event in events {
        let (next, effects) = step(state, event, Stamp(0), &CompanionParams::default());
        state = next;
        all.push(effects);
    }
    all
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
fn the_picker_opens_when_it_becomes_wanted_and_not_while_it_stays() {
    const CASES: &[(&str, Picker, Picker, bool)] = &[
        ("becomes wanted", Picker::Unneeded, Picker::Wanted, true),
        ("stays wanted", Picker::Wanted, Picker::Wanted, false),
        (
            "stops being wanted",
            Picker::Wanted,
            Picker::Unneeded,
            false,
        ),
        ("never wanted", Picker::Unneeded, Picker::Unneeded, false),
    ];
    for (name, before, after, opened) in CASES {
        let want = if *opened {
            vec![CompanionEffect::PickModel]
        } else {
            Vec::new()
        };
        assert_eq!(opens(*before, *after), want, "{name}");
    }
}

#[test]
fn only_an_answer_asked_here_that_lacks_a_model_wants_the_picker() {
    type Case = (&'static str, Companion, Picker);
    let cases: Vec<Case> = vec![
        (
            "asked here, no model",
            held("find the receipts", no_model()),
            Picker::Wanted,
        ),
        (
            "found on the bus, no model",
            held("", no_model()),
            Picker::Unneeded,
        ),
        (
            "asked here, answered",
            held("find the receipts", text()),
            Picker::Unneeded,
        ),
        ("no answer at all", Companion::default(), Picker::Unneeded),
    ];
    for (name, state, want) in cases {
        assert_eq!(picker_of(&state, &task()), want, "{name}");
    }
}

#[test]
fn one_refusal_opens_the_picker_once_however_often_it_is_written() {
    let effects = effects_of(vec![
        asked(),
        answer(wire(
            AnswerPhase::Thinking,
            AnswerBody::Text { lines: Vec::new() },
        )),
        answer(no_model()),
        answer(no_model()),
        CompanionEvent::Session {
            task: task(),
            session: None,
        },
    ]);
    let opened: Vec<usize> = effects
        .iter()
        .enumerate()
        .filter(|(_, effects)| effects.contains(&CompanionEffect::PickModel))
        .map(|(at, _)| at)
        .collect();
    assert_eq!(opened, [2], "only the first write of the refusal opens it");
}

#[test]
fn a_refusal_read_before_the_ask_is_told_still_opens_the_picker_once() {
    let effects = effects_of(vec![answer(no_model()), asked(), answer(no_model())]);
    assert_eq!(
        effects,
        vec![Vec::new(), vec![CompanionEffect::PickModel], Vec::new()]
    );
}

#[test]
fn a_refusal_of_an_answer_nobody_here_asked_opens_nothing() {
    assert_eq!(effects_of(vec![answer(no_model())]), vec![Vec::new()]);
}

#[test]
fn the_turn_that_met_no_model_wears_no_words() {
    let cases = [("failed", AnswerPhase::Failed), ("done", AnswerPhase::Done)];
    for (name, phase) in cases {
        let state = held(
            "find the receipts",
            wire(phase, AnswerBody::Refused(RefusalWire::NoModel)),
        );
        let (title, subtitle) = answer_line(&state.answers[&task()]);
        assert_eq!(
            (title.as_str(), subtitle.as_str()),
            ("find the receipts", ""),
            "{name}"
        );
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
