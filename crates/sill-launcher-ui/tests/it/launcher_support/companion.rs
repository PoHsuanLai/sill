//! The companion's state as the launcher's tests seed it: a finished draft with a button, an
//! answer still being made, the summons a panel can be in.

use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, CardWire, FooterWire};
use docket_core::{
    ActionRef, Args, CallRequest, CardActionId, ContextKeep, Keep, LabelText, Origin, Reveal,
    TargetValue,
};
use prov::{ActionName, AppName, Effect, TaskId};
use sill_model::companion::{
    AnswerFacts, Busy, Companion, CompanionLink, HelpersModel, HelpersMsg, LauncherOrigin, Summon,
    update_helpers,
};

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
            progress: Vec::new(),
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
        routing: Vec::new(),
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

/// `answer` as it was reached through `notes` (the JSON the `Routing` property carries).
pub fn routed(answer: AnswerFacts, notes: &str) -> AnswerFacts {
    AnswerFacts {
        routing: serde_json::from_str(notes).unwrap_or_else(|e| panic!("{e}")),
        ..answer
    }
}

/// The notes of one answering stage on a hosted model.
pub const HAIKU: &str = r#"[{"stage":"answer","served":{"account":"openrouter","model":"claude-haiku-4-5","locality":{"kind":"cloud","v":{"region":null}}},"why":[{"kind":"warm"}],"reached":{"provider":"openrouter","door":"gateway"}}]"#;

/// The notes of an answer the person's chosen coding agent gave: named by the label it goes by.
pub const CLAUDE_CODE: &str = r#"[{"stage":"answer","served":{"account":"agent","model":"claude-code","locality":{"kind":"on_device"}},"name":"Claude Code","why":[{"kind":"named"}],"reached":null}]"#;

/// An answer that failed because the named model could not serve, with the reason as the
/// companion words it.
pub fn declined(why: &str) -> AnswerFacts {
    answer(
        "t-3",
        "ask Kimi",
        AnswerPhase::Failed,
        AnswerBody::Refused(companion_wire::RefusalWire::Failed(why.to_owned())),
    )
}

const RESEARCH: &str = r#"{"id":"h-1","name":"Research","space":"desktop","doing":"Waiting for your answer","state":"working","needs_you":"answer","agent":null}"#;
const INBOX: &str = r#"{"id":"h-2","name":"Inbox","space":"desktop","doing":"Paused","state":"paused","needs_you":null,"agent":null}"#;

/// Two helpers: Research, waiting for the person and with no model, and Inbox, paused. Research's
/// card is the one the model holds open, with one finished job and `more` messages applied after
/// it was read.
pub fn helpers(more: Vec<HelpersMsg>) -> Companion {
    let list = format!(
        r#"{{"helpers":[{RESEARCH},{INBOX}],"told":[{{"n":1,"helper":"h-1","text":"Found the digest."}}]}}"#
    );
    let card = format!(
        r#"{{"line":{RESEARCH},"role":"Reads the news each morning","model":{{"kind":"unset"}},"reach":["org.quire.Notes: look at things"],"steps":12,"jobs":[{{"id":1,"goal":"Find the weekly digest","state":"done","note":null}}]}}"#
    );
    let first = vec![
        HelpersMsg::Listed(
            serde_json::from_str::<companion_wire::HelpersView>(&list)
                .unwrap_or_else(|e| panic!("{e}")),
        ),
        HelpersMsg::OpenCard(prov::HelperId::parse("h-1").unwrap_or_else(|e| panic!("{e}"))),
        HelpersMsg::CardRead(Box::new(
            serde_json::from_str::<companion_wire::HelperCardView>(&card)
                .unwrap_or_else(|e| panic!("{e}")),
        )),
    ];
    let helpers = first
        .into_iter()
        .chain(more)
        .fold(HelpersModel::default(), |model, msg| {
            update_helpers(model, msg).0
        });
    Companion {
        link: CompanionLink::Ready,
        summon: fresh(),
        helpers,
        ..Companion::default()
    }
}
