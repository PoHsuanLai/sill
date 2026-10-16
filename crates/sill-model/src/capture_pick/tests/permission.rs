use super::super::permission::{BODY_MAX_CHARS, BUTTON_MAX_CHARS, LABEL_MAX_CHARS};
use super::super::*;

fn asking(id: u64) -> PermissionAsking {
    PermissionAsking {
        id: AskId(id),
        ask: PermissionAsk::from_wire("Notes", "Share?", "", "", "", ""),
    }
}

#[test]
fn the_words_are_one_clean_line_each_and_the_buttons_have_a_default() {
    // `(case, the wire's six texts, the question made of them)`.
    type Texts = [&'static str; 6];
    let cases: [(&str, Texts, [&str; 6]); 4] = [
        (
            "plain words pass",
            [
                "Notes",
                "Share?",
                "Notes wants it",
                "To work.",
                "Share",
                "Not Now",
            ],
            [
                "Notes",
                "Share?",
                "Notes wants it",
                "To work.",
                "Share",
                "Not Now",
            ],
        ),
        (
            "empty buttons take the usual words",
            ["Notes", "Share?", "", "", "", ""],
            ["Notes", "Share?", "", "", "Allow", "Don\u{2019}t Allow"],
        ),
        (
            "breaks become spaces and control and direction marks go",
            [
                "No\u{202e}tes\u{7}",
                "Line\none\r\ntwo",
                "\u{1b}[31mred",
                "a\tb\u{200f}",
                " \n ",
                "\u{0}",
            ],
            [
                "Notes",
                "Line one  two",
                "[31mred",
                "a b",
                "Allow",
                "Don\u{2019}t Allow",
            ],
        ),
        (
            "a button that is only spaces takes the usual words",
            ["N", "T", "S", "B", "   ", "\t"],
            ["N", "T", "S", "B", "Allow", "Don\u{2019}t Allow"],
        ),
    ];
    for (name, [label, title, subtitle, body, grant, deny], want) in cases {
        let ask = PermissionAsk::from_wire(label, title, subtitle, body, grant, deny);
        let got = [
            ask.app_label.as_str(),
            ask.title.as_str(),
            ask.subtitle.as_str(),
            ask.body.as_str(),
            ask.grant_label.as_str(),
            ask.deny_label.as_str(),
        ];
        assert_eq!(got, want, "{name}");
    }
}

#[test]
fn every_text_is_cut_to_its_ceiling_with_a_mark() {
    let long = "x".repeat(5000);
    let ask = PermissionAsk::from_wire(&long, &long, &long, &long, &long, &long);
    let counts = [
        (ask.app_label.chars().count(), LABEL_MAX_CHARS),
        (ask.body.chars().count(), BODY_MAX_CHARS),
        (ask.grant_label.chars().count(), BUTTON_MAX_CHARS),
        (ask.deny_label.chars().count(), BUTTON_MAX_CHARS),
    ];
    for (got, max) in counts {
        assert_eq!(got, max + 1, "the ceiling and one mark for the cut");
    }
    assert!(ask.grant_label.ends_with('\u{2026}'));
}

#[test]
fn the_outcome_is_the_number_the_call_returns() {
    const CASES: &[(PermissionOutcome, u32)] = &[
        (PermissionOutcome::Grant, 0),
        (PermissionOutcome::Deny, 1),
        (PermissionOutcome::Closed, 2),
    ];
    for &(outcome, want) in CASES {
        assert_eq!(outcome.wire(), want, "{outcome:?}");
    }
}

#[test]
fn a_question_is_one_at_a_time_of_any_kind_and_its_answer_frees_the_sheet() {
    let (open, _) = step(
        CapturePicker::default(),
        CaptureEvent::PermissionOpened(asking(1)),
    );
    assert_eq!(admits(&open), Admit::Busy);
    // A second question of any kind changes nothing.
    let (same, out) = step(open.clone(), CaptureEvent::PermissionOpened(asking(2)));
    assert_eq!((&same, out.is_empty()), (&open, true));
    let sources = Asking {
        id: AskId(3),
        ask: Ask::from_wire("Meet", 1, 0, &[1]).expect("a valid ask"), // test-only
    };
    let (same, _) = step(open.clone(), CaptureEvent::Opened(sources));
    assert_eq!(same, open);

    // A stale answer is ignored; the right one returns the outcome and frees the sheet.
    let (same, out) = step(
        open.clone(),
        CaptureEvent::PermissionAnswered(AskId(9), PermissionOutcome::Grant),
    );
    assert_eq!((&same, out.is_empty()), (&open, true));
    let (freed, out) = step(
        open,
        CaptureEvent::PermissionAnswered(AskId(1), PermissionOutcome::Deny),
    );
    assert_eq!(
        out,
        vec![CaptureOut::PermissionReply(
            AskId(1),
            PermissionOutcome::Deny
        )]
    );
    assert_eq!(admits(&freed), Admit::Yes);
}

#[test]
fn the_caller_leaving_takes_the_question_down() {
    let (open, _) = step(
        CapturePicker::default(),
        CaptureEvent::PermissionOpened(asking(1)),
    );
    let (gone, out) = step(open, CaptureEvent::CallerGone);
    assert_eq!((gone.permission, out.is_empty()), (None, true));
}
