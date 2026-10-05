//! The companion's state as the launcher's tests seed it: a finished draft with a button, an
//! answer still being made, the summons a panel can be in.

#![allow(dead_code)] // Each test binary uses its own subset.

use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, CardWire, FooterWire};
use docket_core::{
    ActionRef, Args, CallRequest, CardActionId, ContextKeep, Keep, LabelText, Origin, Reveal,
    TargetValue,
};
use prov::{ActionName, AppName, Effect, TaskId};
use sill_model::companion::{AnswerFacts, Busy, Companion, CompanionLink, LauncherOrigin, Summon};

pub fn task(id: &str) -> TaskId {
    TaskId::parse(id).unwrap_or_else(|e| panic!("{e}"))
}

pub fn card(id: &str) -> CardWire {
    CardWire {
        id: CardActionId::parse(id).unwrap_or_else(|e| panic!("{e}")),
        label: LabelText::parse("Send").unwrap_or_else(|e| panic!("{e}")),
        effect: Effect::Outbound,
        call: CallRequest {
            action: ActionRef {
                app: AppName::parse("org.quire.Mail").unwrap_or_else(|e| panic!("{e}")),
                name: ActionName::parse("mail.message.send").unwrap_or_else(|e| panic!("{e}")),
            },
            target: TargetValue::Nothing,
            args: Args::new(),
            origin: Origin::Companion,
        },
    }
}

pub fn answer(id: &str, prompt: &str, phase: AnswerPhase, body: AnswerBody) -> AnswerFacts {
    AnswerFacts {
        prompt: prompt.to_owned(),
        wire: AnswerWire {
            task: task(id),
            phase,
            body,
            footer: FooterWire {
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
        shown: std::collections::BTreeMap::new(),
    }
}

pub fn draft() -> AnswerFacts {
    answer(
        "t-1",
        "reply to Sam",
        AnswerPhase::Done,
        AnswerBody::DraftReply {
            to: Vec::new(),
            subject: Reveal::Plain("Re: lunch".to_owned()),
            body: Reveal::Plain("Yes.".to_owned()),
            actions: vec![card("send")],
        },
    )
}

pub fn thinking() -> AnswerFacts {
    answer(
        "t-2",
        "what is on today",
        AnswerPhase::Thinking,
        AnswerBody::Text { lines: Vec::new() },
    )
}

pub fn companion(answers: Vec<AnswerFacts>, summon: Summon) -> Companion {
    let front = answers.last().map(|a| a.wire.task.clone());
    Companion {
        link: CompanionLink::Ready,
        summon,
        front,
        answers: answers
            .into_iter()
            .map(|a| (a.wire.task.clone(), a))
            .collect(),
        ..Companion::default()
    }
}

pub fn fresh() -> Summon {
    Summon::InLauncher {
        origin: LauncherOrigin::Fresh { window: None },
    }
}
