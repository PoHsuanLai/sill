use voice_wire::{
    CancelCause, HeardSegment, HeardTail, HeardText, Level, UtteranceEnd, VoiceEvent as Frame,
    VoiceFault, VoiceRefusal, VoiceTrigger,
};

use super::*;

fn ready() -> Voice {
    Voice {
        link: VoiceLink::Ready,
        ..Voice::default()
    }
}

fn started() -> VoiceEvent {
    VoiceEvent::Told(VoiceCommand::Start {
        scope: AskScope::default(),
        trigger: VoiceTrigger::HoldKey,
    })
}

fn told(command: VoiceCommand) -> VoiceEvent {
    VoiceEvent::Told(command)
}

fn run(mut state: Voice, events: Vec<VoiceEvent>) -> (Voice, Vec<VoiceEffect>) {
    let mut all = Vec::new();
    for event in events {
        let (next, effects) = step(state, event);
        state = next;
        all.extend(effects);
    }
    (state, all)
}

fn partial(text: &str) -> VoiceEvent {
    VoiceEvent::Frame(Frame::Partial(HeardTail {
        text: HeardText(text.to_owned()),
    }))
}

fn committed(text: &str) -> VoiceEvent {
    VoiceEvent::Frame(Frame::Committed(HeardSegment {
        text: HeardText(text.to_owned()),
    }))
}

fn served() -> porter_infer::ServedBy {
    porter_infer::ServedBy {
        account: porter_core::AccountId::parse("local").unwrap_or_else(|e| panic!("{e}")),
        model: porter_core::ModelId::parse("nemotron").unwrap_or_else(|e| panic!("{e}")),
        locality: porter_core::Locality::OnDevice,
    }
}

fn heard_end(text: &str) -> UtteranceEnd {
    UtteranceEnd::Heard {
        text: HeardText(text.to_owned()),
        served: served(),
    }
}

fn live(state: &Voice) -> String {
    match &state.phase {
        Phase::Live(heard) | Phase::Finishing(heard) => heard.text(),
        other => panic!("not listening: {other:?}"),
    }
}

#[test]
fn a_hold_begins_only_where_the_settings_and_the_link_allow() {
    type Case = (&'static str, Voice, bool, Notice);
    let params = |hold_to_talk, consent| VoiceParams {
        hold_to_talk,
        consent,
    };
    let cases: Vec<Case> = vec![
        ("ready, nothing asked", ready(), true, Notice::None),
        (
            "consent given",
            Voice {
                params: params(
                    HoldToTalk::HoldCommand,
                    VoiceConsent::Given(prov::UnixSeconds(5)),
                ),
                ..ready()
            },
            true,
            Notice::None,
        ),
        (
            "consent declined: no begin",
            Voice {
                params: params(HoldToTalk::HoldCommand, VoiceConsent::Declined),
                ..ready()
            },
            false,
            Notice::None,
        ),
        (
            "hold to talk off: no begin",
            Voice {
                params: params(HoldToTalk::Off, VoiceConsent::NotAsked),
                ..ready()
            },
            false,
            Notice::None,
        ),
        (
            "no seat: told so",
            Voice::default(),
            false,
            Notice::Unreachable,
        ),
    ];
    for (name, state, begins, notice) in cases {
        let (next, effects) = step(state, started());
        assert_eq!(
            effects == vec![VoiceEffect::Begin(VoiceTrigger::HoldKey)],
            begins,
            "{name}"
        );
        assert_eq!(next.notice, notice, "{name}");
        assert_eq!(
            matches!(next.phase, Phase::Opening(Then::Listen)),
            begins,
            "{name}"
        );
    }
}

#[test]
fn words_gather_from_partials_and_commits_and_the_end_asks_the_companion() {
    let (state, effects) = run(
        ready(),
        vec![
            started(),
            VoiceEvent::Began,
            partial("open"),
            committed("open the"),
            partial("pod bay"),
        ],
    );
    assert_eq!(live(&state), "open the pod bay");
    assert_eq!(effects, vec![VoiceEffect::Begin(VoiceTrigger::HoldKey)]);
    let (state, effects) = run(
        state,
        vec![
            told(VoiceCommand::Stop),
            VoiceEvent::Frame(Frame::Ended(heard_end("open the pod bay doors"))),
        ],
    );
    assert_eq!(state.phase, Phase::Idle);
    assert_eq!(effects.len(), 2);
    assert_eq!(effects[0], VoiceEffect::Release);
    let VoiceEffect::Ask(ask) = &effects[1] else {
        panic!("{effects:?}");
    };
    assert_eq!(ask.prompt, "open the pod bay doors");
}

#[test]
fn a_levels_frame_changes_nothing_the_person_sees() {
    let (state, _) = run(ready(), vec![started(), VoiceEvent::Began, partial("hi")]);
    let (after, effects) = step(state.clone(), VoiceEvent::Frame(Frame::Level(Level(300))));
    assert_eq!((after, effects), (state, Vec::new()));
}

#[test]
fn the_end_signal_with_words_waits_for_the_stream_and_any_other_end_finishes() {
    let listening = vec![started(), VoiceEvent::Began, partial("hi")];
    let (state, _) = run(ready(), listening.clone());
    let (after, effects) = step(state, VoiceEvent::Ended(heard_end("")));
    assert!(matches!(after.phase, Phase::Live(_)) && effects.is_empty());
    type Case = (&'static str, UtteranceEnd, Notice);
    let cases: Vec<Case> = vec![
        ("nothing", UtteranceEnd::NothingHeard, Notice::NothingHeard),
        (
            "cancelled",
            UtteranceEnd::Cancelled(CancelCause::Shell),
            Notice::None,
        ),
        (
            "no mic",
            UtteranceEnd::Failed(VoiceFault::MicUnavailable),
            Notice::Failed(VoiceFault::MicUnavailable),
        ),
    ];
    for (name, end, notice) in cases {
        let (state, effects) = run(
            ready(),
            [listening.clone(), vec![VoiceEvent::Ended(end)]].concat(),
        );
        assert_eq!(state.phase, Phase::Idle, "{name}");
        assert_eq!(state.notice, notice, "{name}");
        assert_eq!(effects.len(), 1, "{name}: only the begin");
    }
}

#[test]
fn a_hold_let_go_before_begin_returns_releases_when_it_does() {
    let (state, effects) = run(
        ready(),
        vec![started(), told(VoiceCommand::Stop), VoiceEvent::Began],
    );
    assert!(matches!(state.phase, Phase::Finishing(_)));
    assert_eq!(effects[1..], [VoiceEffect::Release]);
    let (state, effects) = run(
        ready(),
        vec![started(), told(VoiceCommand::Cancel), VoiceEvent::Began],
    );
    assert_eq!(state.phase, Phase::Idle);
    assert_eq!(effects[1..], [VoiceEffect::Cancel]);
}

#[test]
fn cancel_discards_the_words_and_asks_nobody() {
    let (state, effects) = run(
        ready(),
        vec![
            started(),
            VoiceEvent::Began,
            partial("never mind"),
            told(VoiceCommand::Cancel),
            VoiceEvent::Frame(Frame::Ended(heard_end("never mind"))),
        ],
    );
    assert_eq!(state.phase, Phase::Idle);
    assert_eq!(state.notice, Notice::None);
    assert_eq!(effects[1..], [VoiceEffect::Cancel]);
}

#[test]
fn a_refusal_names_itself_and_consent_opens_the_sheet() {
    for refusal in VoiceRefusal::ALL {
        let (state, _) = run(ready(), vec![started(), VoiceEvent::Refused(refusal)]);
        assert_eq!(state.phase, Phase::Idle, "{refusal:?}");
        match refusal {
            VoiceRefusal::NeedsConsent => {
                assert_eq!(state.sheet, ConsentSheet::Asking);
                assert_eq!(state.notice, Notice::None);
            }
            other => {
                assert_eq!(state.sheet, ConsentSheet::Closed);
                assert_eq!(state.notice, Notice::Refused(other));
                assert!(!notice_line(state.notice).is_empty(), "{other:?}");
            }
        }
    }
    assert_eq!(
        notice_line(Notice::Refused(VoiceRefusal::MicUnavailable)),
        "No microphone is available."
    );
}

#[test]
fn an_answered_sheet_closes_when_the_settings_say_so() {
    let (asking, _) = run(
        ready(),
        vec![started(), VoiceEvent::Refused(VoiceRefusal::NeedsConsent)],
    );
    let unchanged = VoiceParams::default();
    let (still, _) = step(asking.clone(), told(VoiceCommand::Params(unchanged)));
    assert_eq!(still.sheet, ConsentSheet::Asking);
    for consent in [
        VoiceConsent::Given(prov::UnixSeconds(9)),
        VoiceConsent::Declined,
    ] {
        let params = VoiceParams {
            consent,
            ..unchanged
        };
        let (closed, _) = step(asking.clone(), told(VoiceCommand::Params(params)));
        assert_eq!(closed.sheet, ConsentSheet::Closed, "{consent:?}");
    }
}

#[test]
fn an_answer_closes_the_sheet_and_is_recorded_once() {
    let (asking, _) = run(
        ready(),
        vec![started(), VoiceEvent::Refused(VoiceRefusal::NeedsConsent)],
    );
    for answer in [ConsentAnswer::Allow, ConsentAnswer::NotNow] {
        let (closed, effects) = step(asking.clone(), told(VoiceCommand::Answer(answer)));
        assert_eq!(closed.sheet, ConsentSheet::Closed, "{answer:?}");
        assert_eq!(effects, vec![VoiceEffect::Record(answer)], "{answer:?}");
        let (_, again) = step(closed, told(VoiceCommand::Answer(answer)));
        assert!(again.is_empty(), "{answer:?}: a second answer has no sheet");
    }
}

#[test]
fn the_field_says_listening_then_the_words_then_the_notice() {
    let (state, _) = run(ready(), vec![started(), VoiceEvent::Began]);
    assert_eq!(
        listening_line(&state.phase, state.notice).as_deref(),
        Some("Listening…")
    );
    let (state, _) = step(state, partial("turn it up"));
    assert_eq!(
        listening_line(&state.phase, state.notice).as_deref(),
        Some("turn it up")
    );
    let (state, _) = step(state, VoiceEvent::Ended(UtteranceEnd::NothingHeard));
    assert_eq!(
        listening_line(&state.phase, state.notice).as_deref(),
        Some("Didn't catch that.")
    );
    assert_eq!(listening_line(&Phase::Idle, Notice::None), None);
}

#[test]
fn the_words_never_reach_a_debug_line() {
    let words = Words("my password is hunter2".to_owned());
    assert!(!format!("{words:?}").contains("hunter2"));
}

#[test]
fn consent_is_written_as_the_forms_voiced_reads() {
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct Table {
        consent: VoiceConsent,
    }
    type Case = (VoiceConsent, &'static str);
    const CASES: &[Case] = &[
        (VoiceConsent::NotAsked, "consent = \"not_asked\"\n"),
        (VoiceConsent::Declined, "consent = \"declined\"\n"),
        (
            VoiceConsent::Given(prov::UnixSeconds(1_760_000_000)),
            "[consent]\ngiven = 1760000000\n",
        ),
    ];
    for (consent, text) in CASES {
        let table = Table { consent: *consent };
        assert_eq!(
            toml::to_string(&table).unwrap_or_else(|e| panic!("{e}")),
            *text
        );
        assert_eq!(
            toml::from_str::<Table>(text).unwrap_or_else(|e| panic!("{e}")),
            table
        );
    }
}
