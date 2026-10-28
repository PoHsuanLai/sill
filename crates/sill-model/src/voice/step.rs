//! The voice service's one transition: the hold's phases, the words heard, and what a refusal
//! or an end makes of them.

use docket_core::{ContextKeep, Keep, TurnVia};
use prov::SpaceId;
use voice_wire::{CancelCause, UtteranceEnd, VoiceEvent as Frame, VoiceRefusal};

use super::command::{VoiceCommand, VoiceEffect, VoiceEvent};
use super::model::{AskScope, Heard, HoldToTalk, Notice, Phase, Then, Voice, VoiceConsent, Words};
use crate::companion::AskInput;

/// `state` after `event`, and what the outside must do.
pub fn step(state: Voice, event: VoiceEvent) -> (Voice, Vec<VoiceEffect>) {
    match event {
        VoiceEvent::Told(command) => told(state, command),
        VoiceEvent::Reached(reached) => (Voice { reached, ..state }, Vec::new()),
        VoiceEvent::Began => began(state),
        VoiceEvent::Refused(refusal) => refused(state, refusal),
        VoiceEvent::Unreachable => idle(state, Notice::Unreachable),
        VoiceEvent::Frame(frame) => framed(state, frame),
        // The signal has no words: a heard end waits for the stream's.
        VoiceEvent::Ended(UtteranceEnd::Heard { .. }) => (state, Vec::new()),
        VoiceEvent::Ended(end) => finished(state, end),
        VoiceEvent::StreamClosed => match state.phase {
            Phase::Live(_) | Phase::Finishing(_) => idle(state, Notice::None),
            Phase::Idle | Phase::Opening(_) => (state, Vec::new()),
        },
        // The stream cannot be followed: tell voiced to let go of the microphone.
        VoiceEvent::StreamBroken => match state.phase {
            Phase::Live(_) | Phase::Finishing(_) => {
                let (state, _) = idle(state, Notice::Broken);
                (state, vec![VoiceEffect::Cancel(CancelCause::Shell)])
            }
            Phase::Idle | Phase::Opening(_) => (state, Vec::new()),
        },
        // Only a hold that is over says so: a newer hold is not spoiled by the old cancel.
        VoiceEvent::CancelRefused => match state.phase {
            Phase::Idle => idle(state, Notice::Broken),
            Phase::Opening(_) | Phase::Live(_) | Phase::Finishing(_) => (state, Vec::new()),
        },
    }
}

fn told(state: Voice, command: VoiceCommand) -> (Voice, Vec<VoiceEffect>) {
    match command {
        VoiceCommand::Params(params) => {
            let sheet = match params.consent {
                VoiceConsent::NotAsked => state.sheet,
                VoiceConsent::Given(_) | VoiceConsent::Declined => false,
            };
            (
                Voice {
                    params,
                    sheet,
                    ..state
                },
                Vec::new(),
            )
        }
        VoiceCommand::Start { scope, trigger } => start(state, scope, trigger),
        VoiceCommand::Stop => stop(state),
        VoiceCommand::Cancel(cause) => cancel(state, cause),
        VoiceCommand::Answer(answer) => match state.sheet {
            true => (
                Voice {
                    sheet: false,
                    ..state
                },
                vec![VoiceEffect::Record(answer)],
            ),
            false => (state, Vec::new()),
        },
        VoiceCommand::Dismiss => (
            Voice {
                notice: Notice::None,
                ..state
            },
            Vec::new(),
        ),
    }
}

fn start(
    state: Voice,
    scope: AskScope,
    trigger: voice_wire::VoiceTrigger,
) -> (Voice, Vec<VoiceEffect>) {
    let allowed = matches!(state.params.hold_to_talk, HoldToTalk::HoldCommand)
        && !matches!(state.params.consent, VoiceConsent::Declined);
    match (&state.phase, state.reached, allowed) {
        (Phase::Idle, true, true) => (
            Voice {
                phase: Phase::Opening(Then::Listen),
                scope,
                notice: Notice::None,
                ..state
            },
            vec![VoiceEffect::Begin(trigger)],
        ),
        (Phase::Idle, false, true) => idle(state, Notice::Unreachable),
        _ => (state, Vec::new()),
    }
}

fn stop(state: Voice) -> (Voice, Vec<VoiceEffect>) {
    match state.phase {
        Phase::Opening(_) => (
            Voice {
                phase: Phase::Opening(Then::Release),
                ..state
            },
            Vec::new(),
        ),
        Phase::Live(heard) => (
            Voice {
                phase: Phase::Finishing(heard),
                ..state
            },
            vec![VoiceEffect::Release],
        ),
        Phase::Idle | Phase::Finishing(_) => (state, Vec::new()),
    }
}

fn cancel(state: Voice, cause: CancelCause) -> (Voice, Vec<VoiceEffect>) {
    match state.phase {
        Phase::Opening(_) => (
            Voice {
                phase: Phase::Opening(Then::Cancel(cause)),
                ..state
            },
            Vec::new(),
        ),
        Phase::Live(_) | Phase::Finishing(_) => (
            Voice {
                phase: Phase::Idle,
                ..state
            },
            vec![VoiceEffect::Cancel(cause)],
        ),
        Phase::Idle => (state, Vec::new()),
    }
}

fn began(state: Voice) -> (Voice, Vec<VoiceEffect>) {
    let (phase, effects) = match state.phase {
        Phase::Opening(Then::Listen) => (Phase::Live(Heard::default()), Vec::new()),
        Phase::Opening(Then::Release) => (
            Phase::Finishing(Heard::default()),
            vec![VoiceEffect::Release],
        ),
        Phase::Opening(Then::Cancel(cause)) => (Phase::Idle, vec![VoiceEffect::Cancel(cause)]),
        Phase::Idle | Phase::Live(_) | Phase::Finishing(_) => return (state, Vec::new()),
    };
    (Voice { phase, ..state }, effects)
}

fn refused(state: Voice, refusal: VoiceRefusal) -> (Voice, Vec<VoiceEffect>) {
    match refusal {
        VoiceRefusal::NeedsConsent => (
            Voice {
                phase: Phase::Idle,
                sheet: true,
                ..state
            },
            Vec::new(),
        ),
        other => idle(state, Notice::Refused(other)),
    }
}

fn framed(state: Voice, frame: Frame) -> (Voice, Vec<VoiceEffect>) {
    if let Frame::Ended(end) = frame {
        return finished(state, end);
    }
    let (Phase::Live(heard) | Phase::Finishing(heard)) = state.phase.clone() else {
        return (state, Vec::new());
    };
    let heard = match frame {
        Frame::Partial(tail) => Heard {
            tail: Words(tail.text.0),
            ..heard
        },
        Frame::Committed(segment) => Heard {
            committed: Words(joined(&heard.committed.0, &segment.text.0)),
            tail: Words::default(),
        },
        Frame::Opened | Frame::Waiting(_) | Frame::Level(_) | Frame::Ended(_) => heard,
    };
    let phase = match state.phase {
        Phase::Finishing(_) => Phase::Finishing(heard),
        Phase::Idle | Phase::Opening(_) | Phase::Live(_) => Phase::Live(heard),
    };
    (Voice { phase, ..state }, Vec::new())
}

fn joined(before: &str, next: &str) -> String {
    match (before.is_empty(), next.is_empty()) {
        (true, _) => next.to_owned(),
        (_, true) => before.to_owned(),
        (false, false) => format!("{before} {next}"),
    }
}

fn finished(state: Voice, end: UtteranceEnd) -> (Voice, Vec<VoiceEffect>) {
    match (&state.phase, end) {
        (Phase::Idle | Phase::Opening(_), _) => (state, Vec::new()),
        (_, UtteranceEnd::Heard { text, .. }) if !text.0.trim().is_empty() => {
            let ask = ask_of(&state.scope, text.0);
            let (state, _) = idle(state, Notice::None);
            (state, vec![VoiceEffect::Ask(ask)])
        }
        (_, UtteranceEnd::Heard { .. } | UtteranceEnd::NothingHeard) => {
            idle(state, Notice::NothingHeard)
        }
        (_, UtteranceEnd::Cancelled(CancelCause::TooLong)) => idle(state, Notice::TooLong),
        (_, UtteranceEnd::Cancelled(_)) => idle(state, Notice::None),
        (_, UtteranceEnd::Failed(fault)) => idle(state, Notice::Failed(fault)),
    }
}

fn idle(state: Voice, notice: Notice) -> (Voice, Vec<VoiceEffect>) {
    (
        Voice {
            phase: Phase::Idle,
            notice,
            ..state
        },
        Vec::new(),
    )
}

/// The question `text`, asked over what was focused when the hold began: the window is kept as
/// a chip, nothing else is, and the Space is the desktop's.
fn ask_of(scope: &AskScope, text: String) -> AskInput {
    AskInput {
        prompt: text,
        keep: ContextKeep {
            query: Keep::Dropped,
            results: Keep::Dropped,
            selection: Keep::Dropped,
            window: match scope.window {
                Some(_) => Keep::Kept,
                None => Keep::Dropped,
            },
        },
        window: scope.window,
        app: scope.app.clone(),
        space: SpaceId::desktop(),
        via: TurnVia::Spoken,
    }
}
