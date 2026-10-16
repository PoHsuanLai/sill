use casement_protocols::vocab::WireCode;

use super::super::consent::subject_name;
use super::super::consent_words::{ALLOW, ALLOW_ONCE, DONT_ALLOW, UNNAMED_PROGRAM};
use super::super::*;
use crate::apps::{AppIndex, test_app};
use crate::desktop::{Answer, Consent, ConsentId, ConsentSubject, Memory, Power, SubjectKind};

const SETTINGS_LINE: &str = "You can change this later in Settings, under Privacy.";

fn consent(ask: u32, power: Power, kind: SubjectKind, subject: &str, memory: Memory) -> Consent {
    Consent::from_wire(ConsentId::new(ask), power, kind, subject, memory)
}

fn grim() -> Consent {
    consent(
        1,
        Power::Screen,
        SubjectKind::Unrecognised,
        "/home/me/bin/grim",
        Memory::Remembered,
    )
}

fn opened(state: CapturePicker, id: u64, consent: &Consent) -> CapturePicker {
    let ask = consent_ask(consent, &AppIndex::default());
    let asking = PermissionAsking {
        id: AskId(id),
        ask,
        by: AskedBy::Compositor(consent.ask),
    };
    step(state, CaptureEvent::PermissionOpened(asking)).0
}

fn shown(state: &CapturePicker) -> Option<u64> {
    state.permission.as_ref().map(|asking| asking.id.0)
}

#[test]
fn every_power_has_its_headline_and_every_body_ends_with_the_settings_line() {
    // `(power, the headline for a program called Loupe)`, as the wave 2 design words them.
    let cases = [
        (Power::Screen, "would like to see your screen."),
        (Power::Keys, "would like to type and click for you."),
        (Power::TypingHelp, "would like to help you type."),
        (
            Power::Clipboard,
            "would like to see and change what you copy.",
        ),
        (Power::Lock, "would like to lock your screen."),
        (Power::Displays, "would like to change your displays."),
        (Power::Windows, "would like to see and manage your windows."),
    ];
    assert_eq!(cases.len(), Power::ALL.len(), "a power without a headline");
    for (power, wish) in cases {
        for memory in [Memory::Remembered, Memory::ThisTimeOnly] {
            let ask = consent_ask(
                &consent(1, power, SubjectKind::Program, "/usr/bin/loupe", memory),
                &AppIndex::default(),
            );
            let name = format!("\u{201c}loupe\u{201d} {wish}");
            assert_eq!(ask.title, name, "{power:?}");
            assert!(ask.body.starts_with(power.blurb()), "{power:?}");
            assert!(ask.body.ends_with(SETTINGS_LINE), "{power:?} {memory:?}");
            assert_eq!(ask.deny_label, DONT_ALLOW, "{power:?}");
        }
    }
}

#[test]
fn a_program_that_cannot_be_recognised_is_offered_allow_once_and_told_why() {
    let line = "This program can\u{2019}t be recognised, so you will be asked again next time.";
    let cases = [
        (Memory::Remembered, ALLOW, false),
        (Memory::ThisTimeOnly, ALLOW_ONCE, true),
    ];
    for (memory, grant, extra) in cases {
        let ask = consent_ask(
            &consent(1, Power::Keys, SubjectKind::Unrecognised, "/tmp/x", memory),
            &AppIndex::default(),
        );
        assert_eq!(ask.grant_label, grant, "{memory:?}");
        assert_eq!(ask.body.contains(line), extra, "{memory:?}");
    }
    assert_eq!(ALLOW_ONCE, "Allow Once");
}

#[test]
fn a_program_is_named_by_its_entry_only_when_the_desktop_vouches_for_it() {
    let mut loupe = test_app("org.gnome.Loupe", "Image Viewer");
    loupe.flatpak = Some("org.gnome.Loupe".to_owned());
    let mut viewer = test_app("viewer", "Viewer");
    viewer.exec = "/usr/bin/viewer %U".to_owned();
    let cat = test_app("cat", "Cat");
    let index = AppIndex {
        apps: vec![cat, loupe, viewer],
    };
    let subject = |kind, text: &str| ConsentSubject {
        kind,
        text: text.to_owned(),
    };
    let cases = [
        (
            "a root-held image that an entry runs",
            subject(SubjectKind::Program, "/usr/bin/viewer"),
            "Viewer",
        ),
        (
            "an entry that finds its program on the search path",
            subject(SubjectKind::Program, "/usr/bin/cat"),
            "Cat",
        ),
        (
            "a root-held image no entry runs: the file name",
            subject(SubjectKind::Program, "/usr/bin/grim"),
            "grim",
        ),
        (
            "an image the desktop cannot vouch for is not named by an entry",
            subject(SubjectKind::Unrecognised, "/usr/bin/viewer"),
            "viewer",
        ),
        (
            "a sandboxed app: its entry",
            subject(SubjectKind::SandboxedApp, "org.gnome.Loupe"),
            "Image Viewer",
        ),
        (
            "a sandboxed app with no entry: its id",
            subject(SubjectKind::SandboxedApp, "org.other.App"),
            "org.other.App",
        ),
        (
            "nothing known",
            subject(SubjectKind::Unrecognised, ""),
            UNNAMED_PROGRAM,
        ),
    ];
    for (name, who, want) in cases {
        assert_eq!(subject_name(&who, &index), want, "{name}");
    }
}

#[test]
fn a_long_name_is_cut_so_the_headline_stays_whole() {
    let long = format!("/usr/bin/{}", "x".repeat(300));
    let ask = consent_ask(
        &consent(
            1,
            Power::Windows,
            SubjectKind::Program,
            &long,
            Memory::Remembered,
        ),
        &AppIndex::default(),
    );
    assert!(
        ask.title
            .ends_with("would like to see and manage your windows.")
    );
    assert!(ask.title.contains('\u{2026}'));
}

#[test]
fn the_answer_the_compositor_hears_follows_the_button() {
    let cases = [
        (PermissionOutcome::Grant, Answer::Allow),
        (PermissionOutcome::Deny, Answer::Deny),
        (PermissionOutcome::Closed, Answer::Dismissed),
    ];
    for (outcome, want) in cases {
        let state = opened(CapturePicker::default(), 7, &grim());
        let (state, out) = step(state, CaptureEvent::PermissionAnswered(AskId(7), outcome));
        assert_eq!(
            out,
            vec![CaptureOut::ConsentReply(ConsentId::new(1), want)],
            "{outcome:?}"
        );
        assert_eq!(admits(&state), Admit::Yes);
    }
}

#[test]
fn the_compositors_questions_and_the_portals_share_one_line_in_order() {
    let portal = PermissionAsking {
        id: AskId(1),
        ask: PermissionAsk::from_wire("Notes", "Share?", "", "", "", ""),
        by: AskedBy::Portal,
    };
    let (state, _) = step(
        CapturePicker::default(),
        CaptureEvent::PermissionOpened(portal),
    );
    let state = opened(state, 2, &grim());
    let state = opened(
        state,
        3,
        &consent(
            2,
            Power::Clipboard,
            SubjectKind::Program,
            "/usr/bin/wl-copy",
            Memory::Remembered,
        ),
    );
    assert_eq!(shown(&state), Some(1));
    assert_eq!(state.waiting.len(), 2);

    let (state, out) = step(
        state,
        CaptureEvent::PermissionAnswered(AskId(1), PermissionOutcome::Grant),
    );
    assert_eq!(
        out,
        vec![CaptureOut::PermissionReply(
            AskId(1),
            PermissionOutcome::Grant
        )]
    );
    assert_eq!(shown(&state), Some(2));
    let (state, _) = step(
        state,
        CaptureEvent::PermissionAnswered(AskId(2), PermissionOutcome::Deny),
    );
    assert_eq!(shown(&state), Some(3));
}

#[test]
fn a_question_the_compositor_repeats_is_the_one_already_held() {
    let state = opened(CapturePicker::default(), 1, &grim());
    let state = opened(state, 2, &grim());
    assert_eq!((shown(&state), state.waiting.len()), (Some(1), 0));
    // Once it has waited behind another, the repeat still adds nothing.
    let other = consent(
        2,
        Power::Keys,
        SubjectKind::Program,
        "/usr/bin/wtype",
        Memory::Remembered,
    );
    let state = opened(opened(state, 3, &other), 4, &other);
    assert_eq!(state.waiting.len(), 1);
}

#[test]
fn a_withdrawn_question_comes_down_unanswered_and_the_next_takes_the_sheet() {
    let other = consent(
        2,
        Power::Keys,
        SubjectKind::Program,
        "/usr/bin/wtype",
        Memory::Remembered,
    );
    let state = opened(opened(CapturePicker::default(), 1, &grim()), 2, &other);

    // One that is waiting leaves the line.
    let (waiting_gone, out) = step(
        state.clone(),
        CaptureEvent::PermissionWithdrawn(ConsentId::new(2)),
    );
    assert_eq!(
        (
            shown(&waiting_gone),
            waiting_gone.waiting.len(),
            out.is_empty()
        ),
        (Some(1), 0, true)
    );

    // The one on screen comes down without a reply, and the waiting one is shown.
    let (shown_gone, out) = step(state, CaptureEvent::PermissionWithdrawn(ConsentId::new(1)));
    assert_eq!(
        (shown(&shown_gone), shown_gone.waiting.len(), out.is_empty()),
        (Some(2), 0, true)
    );

    // A number nobody holds changes nothing.
    let (same, _) = step(
        shown_gone.clone(),
        CaptureEvent::PermissionWithdrawn(ConsentId::new(9)),
    );
    assert_eq!(same, shown_gone);
}

#[test]
fn the_portal_leaving_takes_down_its_questions_and_not_the_compositors() {
    let portal = PermissionAsking {
        id: AskId(1),
        ask: PermissionAsk::from_wire("Notes", "Share?", "", "", "", ""),
        by: AskedBy::Portal,
    };
    let (state, _) = step(
        CapturePicker::default(),
        CaptureEvent::PermissionOpened(portal),
    );
    let state = opened(state, 2, &grim());
    let (state, out) = step(state, CaptureEvent::CallerGone);
    assert!(out.is_empty());
    assert_eq!((shown(&state), state.waiting.len()), (Some(2), 0));
}
