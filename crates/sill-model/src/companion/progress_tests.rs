//! Which progress line a row shows and which lines the preview lists, by phase and by how many
//! the companion has said.

use std::collections::BTreeMap;

use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, FooterWire, NeedsYou};
use docket_core::{ContextKeep, Keep};
use prov::TaskId;

use super::progress::{ProgressShown, preview_progress, row_progress};
use super::{AnswerFacts, Busy, answer_line};

fn said(phase: AnswerPhase, progress: &[&str]) -> AnswerFacts {
    AnswerFacts {
        prompt: "what is on today".to_owned(),
        wire: AnswerWire {
            task: TaskId::parse("t-1").unwrap_or_else(|e| panic!("{e}")),
            phase,
            body: AnswerBody::Text { lines: Vec::new() },
            progress: progress.iter().map(|line| (*line).to_owned()).collect(),
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
        },
        busy: Busy::Idle,
        session: None,
        routing: Vec::new(),
        shown: BTreeMap::new(),
    }
}

const FIVE: [&str; 5] = [
    "one",
    "two",
    "I'll check **my memory**",
    "four",
    "Looking at your *calendar*",
];

fn question() -> AnswerPhase {
    AnswerPhase::NeedsYou(NeedsYou::Question {
        text: "Which one?".to_owned(),
        choices: Vec::new(),
    })
}

#[test]
fn the_row_shows_the_newest_progress_line_only_while_working() {
    type Case = (&'static str, AnswerFacts, Option<&'static str>);
    let cases: Vec<Case> = vec![
        ("thinking, none", said(AnswerPhase::Thinking, &[]), None),
        (
            "thinking, one",
            said(AnswerPhase::Thinking, &["Checking"]),
            Some("Checking"),
        ),
        (
            "streaming, three: the newest",
            said(AnswerPhase::Streaming, &["a", "b", "c"]),
            Some("c"),
        ),
        (
            "streaming, five: marks stripped",
            said(AnswerPhase::Streaming, &FIVE),
            Some("Looking at your calendar"),
        ),
        (
            "blank entries are not lines",
            said(AnswerPhase::Thinking, &["Checking", "  "]),
            Some("Checking"),
        ),
        ("done hides it", said(AnswerPhase::Done, &FIVE), None),
        ("needs you hides it", said(question(), &FIVE), None),
        ("failed hides it", said(AnswerPhase::Failed, &FIVE), None),
        (
            "cancelled hides it",
            said(AnswerPhase::Cancelled, &FIVE),
            None,
        ),
    ];
    for (name, answer, want) in cases {
        assert_eq!(row_progress(&answer).as_deref(), want, "{name}");
    }
}

#[test]
fn the_rows_second_line_falls_back_to_the_thinking_words() {
    type Case = (&'static str, AnswerFacts, &'static str);
    let cases: Vec<Case> = vec![
        (
            "thinking, none",
            said(AnswerPhase::Thinking, &[]),
            "Thinking",
        ),
        (
            "thinking, said something",
            said(AnswerPhase::Thinking, &["Checking"]),
            "Checking",
        ),
        (
            "streaming, none",
            said(AnswerPhase::Streaming, &[]),
            "Writing",
        ),
        (
            "streaming, said something",
            said(AnswerPhase::Streaming, &["a", "b"]),
            "b",
        ),
        (
            "done, never the working-out",
            said(AnswerPhase::Done, &FIVE),
            "Done",
        ),
    ];
    for (name, answer, want) in cases {
        assert_eq!(answer_line(&answer).1, want, "{name}");
    }
}

#[test]
fn the_preview_lists_the_newest_three_and_counts_the_rest() {
    let shown = |earlier: usize, lines: &[&str]| ProgressShown {
        earlier,
        lines: lines.iter().map(|line| (*line).to_owned()).collect(),
    };
    type Case = (&'static str, AnswerFacts, ProgressShown);
    let cases: Vec<Case> = vec![
        ("none", said(AnswerPhase::Thinking, &[]), shown(0, &[])),
        ("one", said(AnswerPhase::Thinking, &["a"]), shown(0, &["a"])),
        (
            "three fit",
            said(AnswerPhase::Streaming, &["a", "b", "c"]),
            shown(0, &["a", "b", "c"]),
        ),
        (
            "five fold two, newest last, marks kept",
            said(AnswerPhase::Streaming, &FIVE),
            shown(
                2,
                &[
                    "I'll check **my memory**",
                    "four",
                    "Looking at your *calendar*",
                ],
            ),
        ),
        ("done", said(AnswerPhase::Done, &FIVE), shown(0, &[])),
        ("needs you", said(question(), &FIVE), shown(0, &[])),
        ("failed", said(AnswerPhase::Failed, &FIVE), shown(0, &[])),
        (
            "cancelled",
            said(AnswerPhase::Cancelled, &FIVE),
            shown(0, &[]),
        ),
    ];
    for (name, answer, want) in cases {
        assert_eq!(preview_progress(&answer), want, "{name}");
    }
}
