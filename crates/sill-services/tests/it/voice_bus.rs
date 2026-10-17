//! The voice service on a private bus: a stand-in `voiced` owns `org.quire.Voice1`, the shell is
//! the connection the intents service would publish on its seat. Everything waits on the
//! service's published state or on the stand-in's record, never on a clock; a test skips, saying
//! so, without a `dbus-daemon`. Never the person's session bus.

use crate::voice_support::{self, BeginAnswer, CancelAnswer, desk_or_return};
use docket_core::{Keep, TurnVia, VoiceIntent};
use porter_core::{AccountId, Locality, ModelId};
use porter_infer::ServedBy;
use prov::{SpaceId, UnixSeconds};
use sill_launcher::WindowKey;
use sill_model::companion::CompanionCommand;
use sill_model::voice::{
    AskScope, ConsentAnswer, ConsentSheet, Notice, Phase, VoiceCommand, VoiceConsent, VoiceParams,
    listening_line, notice_line,
};
use voice_wire::{
    CancelCause, Envelope, HeardSegment, HeardTail, HeardText, MAX_FRAME, UtteranceEnd, VoiceBegin,
    VoiceEvent as Frame, VoiceRefusal, VoiceTrigger,
};

fn served() -> ServedBy {
    ServedBy {
        account: AccountId::parse("local").unwrap_or_else(|e| panic!("{e}")),
        model: ModelId::parse("nemotron").unwrap_or_else(|e| panic!("{e}")),
        locality: Locality::OnDevice,
    }
}

fn heard(text: &str) -> UtteranceEnd {
    UtteranceEnd::Heard {
        text: HeardText(text.to_owned()),
        served: served(),
    }
}

fn start() -> VoiceCommand {
    VoiceCommand::Start {
        scope: AskScope {
            window: Some(WindowKey(4)),
            app: None,
        },
        trigger: VoiceTrigger::HoldKey,
    }
}

fn line_of(desk: &voice_support::Desk) -> Option<String> {
    let state = desk.voice.state.borrow();
    listening_line(&state.phase, state.notice)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hold_begins_hears_partials_releases_and_asks_the_companion_what_it_heard() {
    let mut desk = desk_or_return!(BeginAnswer::Open);
    desk.seated().await;
    desk.send(start());
    let begin = desk
        .script
        .until(|lines| {
            lines
                .iter()
                .find_map(|l| l.strip_prefix("begin ").map(str::to_owned))
        })
        .await;
    let sent: Envelope<VoiceBegin> = serde_json::from_str(&begin).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(sent.vocab, voice_wire::VoiceVocab::CURRENT);
    assert_eq!(
        sent.body,
        VoiceBegin {
            intent: VoiceIntent::Ask,
            trigger: VoiceTrigger::HoldKey,
            space: SpaceId::desktop(),
        }
    );
    desk.state(|v| matches!(v.phase, Phase::Live(_)).then_some(()))
        .await;
    desk.script.write(&Frame::Opened).await;
    desk.script
        .write(&Frame::Partial(HeardTail {
            text: HeardText("open the".to_owned()),
        }))
        .await;
    desk.state(|v| {
        (listening_line(&v.phase, v.notice).as_deref() == Some("open the")).then_some(())
    })
    .await;
    desk.script
        .write(&Frame::Committed(HeardSegment {
            text: HeardText("open the pod".to_owned()),
        }))
        .await;
    desk.script
        .write(&Frame::Partial(HeardTail {
            text: HeardText("bay".to_owned()),
        }))
        .await;
    desk.state(|v| {
        (listening_line(&v.phase, v.notice).as_deref() == Some("open the pod bay")).then_some(())
    })
    .await;
    assert!(
        desk.asks.try_recv().is_err(),
        "nothing is asked while the key is held"
    );

    desk.send(VoiceCommand::Stop);
    desk.script
        .until(|lines| lines.iter().any(|l| l == "release").then_some(()))
        .await;
    desk.script
        .write(&Frame::Ended(heard("open the pod bay doors")))
        .await;
    let CompanionCommand::Ask(ask) = desk.next_ask().await else {
        panic!("not an ask");
    };
    assert_eq!(ask.prompt, "open the pod bay doors");
    assert_eq!(ask.via, TurnVia::Spoken);
    assert_eq!(ask.window, Some(WindowKey(4)));
    assert_eq!(ask.keep.window, Keep::Kept);
    desk.state(|v| matches!(v.phase, Phase::Idle).then_some(()))
        .await;
    assert_eq!(line_of(&desk), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_end_signal_that_beats_the_reply_to_begin_is_not_lost() {
    let mut desk = desk_or_return!(BeginAnswer::Open);
    *desk
        .script
        .early_end
        .lock()
        .unwrap_or_else(|e| panic!("{e}")) = Some(UtteranceEnd::Cancelled(CancelCause::Shell));
    desk.seated().await;
    desk.send(start());
    desk.script
        .until(|lines| lines.iter().any(|l| l.starts_with("begin ")).then_some(()))
        .await;
    // The stream stays silent: only the signal, sent before `Begin` replied, can end the hold.
    desk.state(|v| matches!(v.phase, Phase::Idle).then_some(()))
        .await;
    assert_eq!(desk.voice.state.borrow().notice, Notice::None);
    assert!(desk.asks.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn needs_consent_opens_the_sheet_and_each_answer_is_the_consent_to_write() {
    for (answer, want) in [
        (ConsentAnswer::NotNow, VoiceConsent::Declined),
        (ConsentAnswer::Allow, VoiceConsent::Given(UnixSeconds(0))),
    ] {
        let mut desk = desk_or_return!(BeginAnswer::Refuse(VoiceRefusal::NeedsConsent));
        desk.seated().await;
        desk.send(start());
        desk.state(|v| matches!(v.sheet, ConsentSheet::Asking).then_some(()))
            .await;
        assert_eq!(desk.voice.state.borrow().notice, Notice::None, "{answer:?}");
        desk.send(VoiceCommand::Answer(answer));
        let consent =
            tokio::time::timeout(std::time::Duration::from_secs(60), desk.consents.recv())
                .await
                .unwrap_or_else(|_| panic!("no consent was sent"))
                .unwrap_or_else(|| panic!("closed"));
        match (consent, want) {
            (VoiceConsent::Given(at), VoiceConsent::Given(_)) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_else(|e| panic!("{e}"))
                    .as_secs();
                assert!(
                    at.0.abs_diff(i64::try_from(now).unwrap_or(0)) < 60,
                    "{at:?}"
                );
            }
            (consent, want) => assert_eq!(consent, want),
        }
        desk.state(|v| matches!(v.sheet, ConsentSheet::Closed).then_some(()))
            .await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_declined_consent_never_calls_begin() {
    let mut desk = desk_or_return!(BeginAnswer::Open);
    desk.seated().await;
    desk.send(VoiceCommand::Params(VoiceParams {
        consent: VoiceConsent::Declined,
        ..VoiceParams::default()
    }));
    desk.send(start());
    // The next hold is told apart from the refused one by the record: a second command that does
    // begin (consent given) must be the first `begin` the stand-in saw.
    desk.send(VoiceCommand::Params(VoiceParams::default()));
    desk.send(start());
    let first = desk
        .script
        .until(|lines| lines.iter().find(|l| l.starts_with("begin ")).cloned())
        .await;
    assert!(first.contains("hold_key"), "{first}");
    assert_eq!(
        desk.script
            .lines()
            .iter()
            .filter(|l| l.starts_with("begin "))
            .count(),
        1,
        "the declined hold made no call: {:?}",
        desk.script.lines()
    );
    assert!(desk.asks.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_missing_microphone_is_said_plainly() {
    let mut desk = desk_or_return!(BeginAnswer::Refuse(VoiceRefusal::MicUnavailable));
    desk.seated().await;
    desk.send(start());
    let notice = desk
        .state(|v| match v.notice {
            Notice::None => None,
            notice => Some(notice),
        })
        .await;
    assert_eq!(notice, Notice::Refused(VoiceRefusal::MicUnavailable));
    assert_eq!(notice_line(notice), "No microphone is available.");
    assert_eq!(
        line_of(&desk).as_deref(),
        Some("No microphone is available.")
    );
    assert_eq!(desk.voice.state.borrow().sheet, ConsentSheet::Closed);
    desk.send(VoiceCommand::Dismiss);
    desk.state(|v| matches!(v.notice, Notice::None).then_some(()))
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_tells_voiced_and_discards_what_was_heard() {
    let mut desk = desk_or_return!(BeginAnswer::Open);
    desk.seated().await;
    desk.send(start());
    desk.state(|v| matches!(v.phase, Phase::Live(_)).then_some(()))
        .await;
    desk.script
        .write(&Frame::Partial(HeardTail {
            text: HeardText("never mind".to_owned()),
        }))
        .await;
    desk.state(|v| {
        (listening_line(&v.phase, v.notice).as_deref() == Some("never mind")).then_some(())
    })
    .await;
    desk.send(VoiceCommand::Cancel(CancelCause::Escape));
    desk.script
        .until(|lines| lines.iter().any(|l| l == "cancel escape").then_some(()))
        .await;
    desk.state(|v| matches!(v.phase, Phase::Idle).then_some(()))
        .await;
    assert_eq!(desk.voice.state.borrow().notice, Notice::None);
    assert!(
        desk.asks.try_recv().is_err(),
        "a cancelled hold asks nobody"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_the_seat_a_hold_says_voice_is_not_running() {
    let desk = desk_or_return!(BeginAnswer::Open);
    desk.send(start());
    desk.state(|v| matches!(v.notice, Notice::Unreachable).then_some(()))
        .await;
    assert!(desk.script.lines().is_empty(), "nothing was said to voiced");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_carries_the_cause_of_each_trigger() {
    for (cause, name) in [
        (CancelCause::Escape, "escape"),
        (CancelCause::OtherInput, "other_input"),
        (CancelCause::FocusLost, "focus_lost"),
        (CancelCause::Shell, "shell"),
    ] {
        let mut desk = desk_or_return!(BeginAnswer::Open);
        desk.seated().await;
        desk.send(start());
        desk.state(|v| matches!(v.phase, Phase::Live(_)).then_some(()))
            .await;
        desk.send(VoiceCommand::Cancel(cause));
        let want = format!("cancel {name}");
        desk.script
            .until(|lines| lines.contains(&want).then_some(()))
            .await;
        desk.state(|v| matches!(v.phase, Phase::Idle).then_some(()))
            .await;
        assert_eq!(desk.voice.state.borrow().notice, Notice::None, "{name}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_malformed_frame_is_skipped_and_the_frames_after_it_arrive() {
    let mut desk = desk_or_return!(BeginAnswer::Open);
    desk.seated().await;
    desk.send(start());
    desk.state(|v| matches!(v.phase, Phase::Live(_)).then_some(()))
        .await;
    let junk = b"{not json";
    let mut bytes = u32::try_from(junk.len())
        .unwrap_or_else(|e| panic!("{e}"))
        .to_be_bytes()
        .to_vec();
    bytes.extend_from_slice(junk);
    desk.script.write_raw(&bytes).await;
    desk.script
        .write(&Frame::Partial(HeardTail {
            text: HeardText("still here".to_owned()),
        }))
        .await;
    desk.state(|v| {
        (listening_line(&v.phase, v.notice).as_deref() == Some("still here")).then_some(())
    })
    .await;
    assert_eq!(desk.voice.state.borrow().notice, Notice::None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_frame_over_the_cap_closes_the_stream_and_says_so_plainly() {
    let mut desk = desk_or_return!(BeginAnswer::Open);
    desk.seated().await;
    desk.send(start());
    desk.state(|v| matches!(v.phase, Phase::Live(_)).then_some(()))
        .await;
    desk.script.write_raw(&(MAX_FRAME + 1).to_be_bytes()).await;
    let notice = desk
        .state(|v| match v.notice {
            Notice::None => None,
            notice => Some(notice),
        })
        .await;
    assert_eq!(notice, Notice::Broken);
    assert_eq!(notice_line(notice), "Voice stopped working.");
    assert!(matches!(desk.voice.state.borrow().phase, Phase::Idle));
    desk.script
        .until(|lines| lines.iter().any(|l| l == "cancel shell").then_some(()))
        .await;
    assert!(desk.asks.try_recv().is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cancel_voiced_refuses_as_malformed_is_told_and_the_next_hold_still_begins() {
    let mut desk = desk_or_return!(BeginAnswer::Open);
    *desk
        .script
        .cancel_answer
        .lock()
        .unwrap_or_else(|e| panic!("{e}")) = CancelAnswer::Malformed;
    desk.seated().await;
    desk.send(start());
    desk.state(|v| matches!(v.phase, Phase::Live(_)).then_some(()))
        .await;
    desk.send(VoiceCommand::Cancel(CancelCause::Escape));
    desk.script
        .until(|lines| lines.iter().any(|l| l == "cancel refused").then_some(()))
        .await;
    desk.state(|v| matches!(v.notice, Notice::Broken).then_some(()))
        .await;
    desk.send(VoiceCommand::Dismiss);
    desk.send(start());
    desk.state(|v| matches!(v.phase, Phase::Live(_)).then_some(()))
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_too_long_end_is_told_and_a_superseded_one_is_not() {
    for (cause, notice) in [
        (CancelCause::TooLong, Notice::TooLong),
        (CancelCause::Superseded, Notice::None),
    ] {
        let mut desk = desk_or_return!(BeginAnswer::Open);
        desk.seated().await;
        desk.send(start());
        desk.state(|v| matches!(v.phase, Phase::Live(_)).then_some(()))
            .await;
        desk.script
            .write(&Frame::Ended(UtteranceEnd::Cancelled(cause)))
            .await;
        desk.state(|v| matches!(v.phase, Phase::Idle).then_some(()))
            .await;
        assert_eq!(desk.voice.state.borrow().notice, notice, "{cause:?}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_program_that_only_owns_the_voice_name_is_not_asked() {
    let desk =
        voice_support::desk_with(BeginAnswer::Open, voice_support::VoicedIs::OnlyOwnsTheName).await;
    let Some(mut desk) = desk else {
        return;
    };
    desk.seated().await;
    desk.send(start());
    desk.state(|v| matches!(v.notice, Notice::Unreachable).then_some(()))
        .await;
    assert!(
        desk.script.lines().is_empty(),
        "nothing was said to a program that is not voiced: {:?}",
        desk.script.lines()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_end_signal_from_a_program_that_is_not_voiced_ends_nothing() {
    let mut desk = desk_or_return!(BeginAnswer::Open);
    desk.seated().await;
    desk.send(start());
    desk.state(|v| matches!(v.phase, Phase::Live(_)).then_some(()))
        .await;
    desk.voiced_becomes(voice_support::VoicedIs::OnlyOwnsTheName);
    desk.end_signal(1, &UtteranceEnd::Cancelled(CancelCause::Shell))
        .await;
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    assert!(
        matches!(desk.voice.state.borrow().phase, Phase::Live(_)),
        "the hold ended on a signal from a program that is not voiced"
    );
}
