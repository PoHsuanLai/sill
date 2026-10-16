use super::super::permission::{BODY_MAX_CHARS, BUTTON_MAX_CHARS, LABEL_MAX_CHARS};
use super::super::*;

fn asking(id: u64) -> PermissionAsking {
    PermissionAsking {
        id: AskId(id),
        ask: PermissionAsk::from_wire("Notes", "Share?", "", "", "", ""),
        by: AskedBy::Portal,
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
fn a_question_takes_the_sheet_and_its_answer_frees_it() {
    let (open, _) = step(
        CapturePicker::default(),
        CaptureEvent::PermissionOpened(asking(1)),
    );
    assert_eq!(open.permission.as_ref().map(|a| a.id), Some(AskId(1)));
    // The sheet is taken: a sharing or colour request is refused at once.
    assert_eq!(admits(&open), Admit::Busy);
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
fn a_question_that_finds_the_sheet_taken_waits_its_turn() {
    let opened = |state, id| step(state, CaptureEvent::PermissionOpened(asking(id))).0;
    let state = opened(opened(opened(CapturePicker::default(), 1), 2), 3);
    let waiting = |state: &CapturePicker| -> Vec<AskId> {
        state.waiting.iter().map(|asking| asking.id).collect()
    };
    assert_eq!(state.permission.as_ref().map(|a| a.id), Some(AskId(1)));
    assert_eq!(waiting(&state), vec![AskId(2), AskId(3)]);

    // Each answer brings the next one up, in the order they came.
    let (state, _) = step(
        state,
        CaptureEvent::PermissionAnswered(AskId(1), PermissionOutcome::Grant),
    );
    assert_eq!(state.permission.as_ref().map(|a| a.id), Some(AskId(2)));
    assert_eq!(waiting(&state), vec![AskId(3)]);
    let (state, _) = step(
        state,
        CaptureEvent::PermissionAnswered(AskId(2), PermissionOutcome::Closed),
    );
    assert_eq!(state.permission.as_ref().map(|a| a.id), Some(AskId(3)));
    assert_eq!(waiting(&state), Vec::<AskId>::new());
}

#[test]
fn the_line_is_capped() {
    let mut state = CapturePicker::default();
    for id in 0..(WAITING_MAX as u64 + 3) {
        state = step(state, CaptureEvent::PermissionOpened(asking(id))).0;
    }
    assert_eq!(state.waiting.len(), WAITING_MAX);
    assert_eq!(permission_admits(&state), Admit::Busy);
}

#[test]
fn a_sharing_request_that_ends_lets_the_waiting_question_up() {
    let sources = Asking {
        id: AskId(1),
        ask: Ask::from_wire("Meet", 1, 0, &[1]).expect("a valid ask"), // test-only
    };
    let (state, _) = step(CapturePicker::default(), CaptureEvent::Opened(sources));
    let (state, _) = step(state, CaptureEvent::PermissionOpened(asking(2)));
    assert_eq!(state.permission, None);
    assert_eq!(state.waiting.len(), 1);
    let (state, _) = step(state, CaptureEvent::Answered(AskId(1), Outcome::Cancel));
    assert_eq!(state.permission.as_ref().map(|a| a.id), Some(AskId(2)));
    assert!(state.waiting.is_empty());
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
