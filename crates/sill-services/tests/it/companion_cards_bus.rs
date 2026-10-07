//! The companion service's answers as the shell follows them, on a private bus: a card press and
//! the step it starts, the confirmation the answer waits on, a step that fails, the text behind a
//! handle, the things an app opens, and what is said to a subagent. Stand-ins own the peers'
//! names; every wait is on the service's published state or on a stand-in's record, never on a
//! clock. Never the person's session bus.

use crate::companion_support::{self, desk_or_return};

use std::sync::{Arc, Mutex};

use companion_support::stand_ins::{
    ActsAs, StandInCompanion, StandInMessage, StandInRegistry, StandInRun, StandInSession,
};
use companion_support::{Desk, app, json, task, wire};
use companion_wire::{
    AnswerBody, AnswerPhase, AnswerWire, CardWire, NeedsYou, PlanStepWire, PlanWire, StepWireState,
};
use docket_core::{
    ActionRef, Args, CallRefusal, CallRequest, CardActionId, ConfirmEnd, ConfirmId, ContextKeep,
    Handle, Keep, LabelText, Manifest, Origin, Reveal, StepId, TargetValue, UserTurn, validate,
};
use docket_dbus::{COMPANION_BUS, INTENTS_BUS, answer_path};
use prov::{ActionName, AgentRef, AppName, Effect, EntityId, EntityKey, EntityKind, SpaceId};
use sill_model::companion::{AskInput, Busy, Companion, CompanionCommand, Opener, answer_line};
use zbus::Connection;

fn mail() -> AppName {
    app("org.quire.Mail")
}

fn send_action() -> ActionRef {
    ActionRef {
        app: mail(),
        name: ActionName::parse("mail.message.send").unwrap_or_else(|e| panic!("{e}")),
    }
}

fn send_id() -> CardActionId {
    CardActionId::parse("send").unwrap_or_else(|e| panic!("{e}"))
}

/// A reply draft that offers the send button.
fn draft_wire(phase: AnswerPhase) -> AnswerWire {
    AnswerWire {
        body: AnswerBody::DraftReply {
            to: Vec::new(),
            subject: Reveal::Plain("Re: lunch".to_owned()),
            body: Reveal::Plain("Yes.".to_owned()),
            actions: vec![CardWire {
                id: send_id(),
                label: LabelText::parse("Send").unwrap_or_else(|e| panic!("{e}")),
                effect: Effect::Outbound,
                call: CallRequest {
                    action: send_action(),
                    target: TargetValue::Nothing,
                    args: Args::new(),
                    origin: Origin::Companion,
                },
            }],
        },
        ..wire(phase, "")
    }
}

/// What the companion writes once the send button is pressed: a plan with that one step.
fn plan_wire(phase: AnswerPhase, state: StepWireState) -> AnswerWire {
    AnswerWire {
        body: AnswerBody::Plan(PlanWire {
            steps: vec![PlanStepWire {
                id: StepId(1),
                action: send_action(),
                label: LabelText::parse("Send").unwrap_or_else(|e| panic!("{e}")),
                effect: Effect::Outbound,
                state,
                call: None,
            }],
        }),
        ..wire(phase, "")
    }
}

fn no_context() -> ContextKeep {
    ContextKeep {
        query: Keep::Dropped,
        results: Keep::Dropped,
        selection: Keep::Dropped,
        window: Keep::Dropped,
    }
}

fn ask_input() -> AskInput {
    AskInput {
        prompt: "draft a reply".to_owned(),
        keep: no_context(),
        window: None,
        app: None,
        space: SpaceId::desktop(),
        via: docket_core::TurnVia::Typed,
    }
}

/// The bench for an answer: stand-in companiond and intentd are up, the shell holds its seat, and
/// an ask has made the answer, whose view is `first`.
struct Bench {
    desk: Desk,
    companiond: Connection,
    /// Held so the stand-in intentd keeps its name while the test runs.
    _intentd: Connection,
    view: Arc<Mutex<String>>,
}

async fn bench(acts: ActsAs, first: &AnswerWire) -> Option<Bench> {
    let mut desk = companion_support::desk().await?;
    let view = Arc::new(Mutex::new(json(first)));
    let companiond = desk.peer(COMPANION_BUS).await;
    companiond
        .object_server()
        .at(
            "/org/quire/Companion1",
            StandInCompanion {
                session: None,
                log: desk.log.clone(),
                view: view.clone(),
                acts,
                routing: None,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let intentd = desk.peer(INTENTS_BUS).await;
    intentd
        .object_server()
        .at("/org/quire/Intents1", StandInSession(desk.log.clone()))
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.seated().await;
    desk.send(CompanionCommand::Ask(ask_input()));
    // The follower reads the view once it has subscribed to `Updated`: when the card is in the
    // state, an update emitted now is not lost.
    desk.state(|c| {
        c.answers
            .get(&task())
            .filter(|a| matches!(a.wire.body, AnswerBody::DraftReply { .. }))
            .map(|_| ())
    })
    .await;
    Some(Bench {
        desk,
        companiond,
        _intentd: intentd,
        view,
    })
}

impl Bench {
    /// The companion says the answer is now `next`.
    async fn updates(&self, next: &AnswerWire) {
        *self.view.lock().unwrap_or_else(|e| panic!("{e}")) = json(next);
        self.companiond
            .emit_signal(
                None::<&str>,
                answer_path(&task()).as_str(),
                "org.quire.Companion1.Answer",
                "Updated",
                &(json(next),),
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// The state once the answer's phase is `phase`.
    async fn phase(&self, phase: AnswerPhase) -> Companion {
        self.desk
            .state(|c| {
                c.answers
                    .get(&task())
                    .filter(|a| a.wire.phase == phase)
                    .map(|_| c.clone())
            })
            .await
    }
}

macro_rules! bench {
    ($acts:expr, $first:expr) => {
        match bench($acts, $first).await {
            Some(bench) => bench,
            None => return,
        }
    };
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_card_press_is_busy_until_its_step_leaves_running() {
    let first = draft_wire(AnswerPhase::Done);
    let bench = bench!(ActsAs::Accepts, &first);
    bench.desk.send(CompanionCommand::Card {
        task: task(),
        action: send_id(),
    });
    let acting = Busy::Acting {
        card: send_id(),
        step: send_action(),
    };
    bench
        .desk
        .state(|c| (c.answers[&task()].busy == acting).then_some(()))
        .await;
    bench.desk.log_starting("act ").await;
    // The step is running: the press is still going.
    bench
        .updates(&plan_wire(AnswerPhase::Streaming, StepWireState::Running))
        .await;
    let running = bench.phase(AnswerPhase::Streaming).await;
    assert_eq!(running.answers[&task()].busy, acting);
    // The step is done: the button is free again.
    bench
        .updates(&plan_wire(
            AnswerPhase::Done,
            StepWireState::Done { undo: None },
        ))
        .await;
    let done = bench.phase(AnswerPhase::Done).await;
    assert_eq!(done.answers[&task()].busy, Busy::Idle);
    assert_eq!(
        bench
            .desk
            .log
            .lines()
            .iter()
            .filter(|l| l.starts_with("act "))
            .count(),
        1,
        "one press, one call"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_second_press_while_one_is_going_is_not_sent() {
    let first = draft_wire(AnswerPhase::Done);
    let bench = bench!(ActsAs::Accepts, &first);
    for _ in 0..2 {
        bench.desk.send(CompanionCommand::Card {
            task: task(),
            action: send_id(),
        });
    }
    bench.desk.log_starting("act ").await;
    // A press that was sent would be in the record by the time this later one's effect is.
    bench
        .updates(&plan_wire(AnswerPhase::Streaming, StepWireState::Running))
        .await;
    bench.phase(AnswerPhase::Streaming).await;
    assert_eq!(
        bench
            .desk
            .log
            .lines()
            .iter()
            .filter(|l| l.starts_with("act "))
            .count(),
        1,
        "the second press found the button busy"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_confirmation_the_answer_waits_on_is_followed_and_ends() {
    let first = draft_wire(AnswerPhase::Done);
    let bench = bench!(ActsAs::Accepts, &first);
    bench.desk.send(CompanionCommand::Card {
        task: task(),
        action: send_id(),
    });
    bench.desk.log_starting("act ").await;
    let confirm = AnswerPhase::NeedsYou(NeedsYou::Confirm(
        ConfirmId::parse("c-7").unwrap_or_else(|e| panic!("{e}")),
    ));
    bench
        .updates(&plan_wire(confirm.clone(), StepWireState::Running))
        .await;
    let waiting = bench.phase(confirm).await;
    assert_eq!(
        answer_line(&waiting.answers[&task()]).1,
        "Needs your confirmation"
    );
    assert_eq!(
        sill_model::companion::presence_of(&waiting),
        ds_core::vocab::CompanionPresence::Waiting,
        "the orb waits on the person while the sheet is up"
    );
    bench
        .updates(&plan_wire(
            AnswerPhase::Done,
            StepWireState::Done { undo: None },
        ))
        .await;
    let done = bench.phase(AnswerPhase::Done).await;
    assert_eq!(
        sill_model::companion::presence_of(&done),
        ds_core::vocab::CompanionPresence::Idle
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_step_that_fails_is_shown_as_failed_while_the_task_is_done() {
    let first = draft_wire(AnswerPhase::Done);
    let bench = bench!(ActsAs::Accepts, &first);
    bench.desk.send(CompanionCommand::Card {
        task: task(),
        action: send_id(),
    });
    bench.desk.log_starting("act ").await;
    let refusal = CallRefusal::Unconfirmed(ConfirmEnd::Refused);
    bench
        .updates(&plan_wire(
            AnswerPhase::Done,
            StepWireState::Failed(refusal.clone()),
        ))
        .await;
    let state = bench
        .desk
        .state(|c| {
            let answer = c.answers.get(&task())?;
            matches!(&answer.wire.body, AnswerBody::Plan(_)).then(|| c.clone())
        })
        .await;
    let answer = &state.answers[&task()];
    assert_eq!(answer.busy, Busy::Idle, "the press ended");
    assert_eq!(
        answer_line(answer).1,
        "A step failed",
        "the task is fine and the card's call was not"
    );
    match &answer.wire.body {
        AnswerBody::Plan(plan) => assert_eq!(plan.steps[0].state, StepWireState::Failed(refusal)),
        other => panic!("{other:?}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_press_the_companion_refuses_is_kept_and_the_button_is_offered_again() {
    let first = draft_wire(AnswerPhase::Done);
    let bench = bench!(ActsAs::Refuses, &first);
    bench.desk.send(CompanionCommand::Card {
        task: task(),
        action: send_id(),
    });
    let state = bench
        .desk
        .state(|c| matches!(c.answers[&task()].busy, Busy::Refused(_)).then(|| c.clone()))
        .await;
    assert_eq!(
        answer_line(&state.answers[&task()]).1,
        "Could not do that",
        "a refusal is said, not swallowed"
    );
    // A refused press leaves the button free: a second press is sent.
    bench.desk.send(CompanionCommand::Card {
        task: task(),
        action: send_id(),
    });
    bench
        .desk
        .log
        .until(|lines| (lines.iter().filter(|l| l.starts_with("act ")).count() == 2).then_some(()))
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_handle_in_an_answer_is_shown_through_session_display_once() {
    let first = draft_wire(AnswerPhase::Done);
    let bench = bench!(ActsAs::Accepts, &first);
    let with_handle = |phase| AnswerWire {
        body: AnswerBody::Text {
            lines: vec![Reveal::Plain("Found".to_owned()), Reveal::Handle(Handle(7))],
        },
        ..wire(phase, "")
    };
    bench.updates(&with_handle(AnswerPhase::Streaming)).await;
    let state = bench
        .desk
        .state(|c| {
            let shown = &c.answers.get(&task())?.shown;
            (!shown.is_empty()).then(|| c.clone())
        })
        .await;
    assert_eq!(
        state.answers[&task()]
            .shown
            .get(&Handle(7))
            .map(String::as_str),
        Some("the text of handle 7")
    );
    assert_eq!(
        answer_line(&state.answers[&task()]).1,
        "Found",
        "the row's line is the text the router showed"
    );
    // The answer changes again with the same handle: the router is not asked a second time.
    bench.updates(&with_handle(AnswerPhase::Done)).await;
    bench.phase(AnswerPhase::Done).await;
    assert_eq!(
        bench
            .desk
            .log
            .lines()
            .iter()
            .filter(|l| l.starts_with("display "))
            .collect::<Vec<_>>(),
        vec!["display s-1 7"],
        "the session the ask opened, the handle the answer holds, once"
    );
}

const MAIL_MANIFEST: &str = r#"
vocab = 1
app = "org.quire.Mail"

[[entities]]
kind = "mail.thread"
label = "Thread"
plural = "Threads"
icon = "mail"
class = "mail"
index = "indexed"
titles = { kind = "third_party", v = { kind = "mail" } }
props = []

[[entities]]
kind = "mail.contact"
label = "Contact"
plural = "Contacts"
icon = "person"
class = "contacts"
index = "not_indexed"
titles = { kind = "app_authored" }
props = []

[[actions]]
name = "mail.thread.open"
label = "Open"
on = { kind = "one", v = "mail.thread" }
effect = "read"
classes = []
undo = "not_undoable"
reach = "hidden"
latency = "quick"
result = { kind = "nothing" }
keys = { kind = "none" }
lasting = "no"
dry_run = "none"
params = []
"#;

fn mail_manifest() -> docket_core::ValidManifest {
    let manifest: Manifest = toml::from_str(MAIL_MANIFEST).unwrap_or_else(|e| panic!("{e}"));
    validate(manifest).unwrap_or_else(|e| panic!("{e:?}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_apps_that_declare_an_open_action_are_known_and_a_perform_is_the_launchers() {
    let mut desk = desk_or_return!();
    let intentd = desk.peer(INTENTS_BUS).await;
    for object in [
        intentd
            .object_server()
            .at(
                "/org/quire/Intents1",
                StandInRegistry(vec![mail_manifest()]),
            )
            .await,
        intentd
            .object_server()
            .at("/org/quire/Intents1", StandInRun(desk.log.clone()))
            .await,
    ] {
        object.unwrap_or_else(|e| panic!("{e}"));
    }
    desk.seated().await;
    let opens = desk
        .state(|c| (!c.openers.is_empty()).then(|| c.openers.clone()))
        .await;
    let thread = EntityId {
        app: mail(),
        kind: EntityKind::parse("mail.thread").unwrap_or_else(|e| panic!("{e}")),
        key: EntityKey::parse("a1").unwrap_or_else(|e| panic!("{e}")),
    };
    assert_eq!(
        opens.into_iter().collect::<Vec<_>>(),
        vec![Opener {
            app: mail(),
            kind: thread.kind.clone(),
        }],
        "the kind with an open action; the contact kind declares none"
    );
    let call = CallRequest {
        action: ActionRef {
            app: mail(),
            name: docket_core::open_name(&thread.kind).unwrap_or_else(|| panic!("a name")),
        },
        target: TargetValue::Entities(vec![thread]),
        args: Args::new(),
        origin: Origin::Launcher,
    };
    desk.send(CompanionCommand::Perform(Box::new(call.clone())));
    let performed = desk.log_starting("perform ").await;
    assert_eq!(
        performed,
        format!("perform {} null null", json(&call)),
        "the open call as the launcher, with no session and no window"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn something_said_to_a_subagent_is_recorded_sent_and_told_to_the_companion() {
    let mut desk = desk_or_return!();
    let view = Arc::new(Mutex::new(json(&wire(AnswerPhase::Done, ""))));
    let companiond = desk.peer(COMPANION_BUS).await;
    companiond
        .object_server()
        .at(
            "/org/quire/Companion1",
            StandInCompanion {
                session: None,
                log: desk.log.clone(),
                view,
                acts: ActsAs::Accepts,
                routing: None,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let intentd = desk.peer(INTENTS_BUS).await;
    for object in [
        intentd
            .object_server()
            .at("/org/quire/Intents1", StandInSession(desk.log.clone()))
            .await,
        intentd
            .object_server()
            .at("/org/quire/Intents1", StandInMessage(desk.log.clone()))
            .await,
    ] {
        object.unwrap_or_else(|e| panic!("{e}"));
    }
    desk.seated().await;
    let worker = AgentRef::Worker { task: task() };
    let work = SpaceId::parse("work").unwrap_or_else(|e| panic!("{e}"));
    desk.send(CompanionCommand::Tell {
        agent: worker.clone(),
        space: work.clone(),
        text: "use the second draft".to_owned(),
    });
    let told = desk.log_starting("told ").await;
    assert_eq!(
        told.split_whitespace().nth(2),
        Some("work"),
        "the Space it works in: {told}"
    );
    let lines = desk.log.lines();
    let position = |prefix: &str| {
        lines
            .iter()
            .position(|l| l.starts_with(prefix))
            .unwrap_or_else(|| panic!("{prefix}: {lines:?}"))
    };
    assert!(
        position("turn ") < position("send ") && position("send ") < position("told "),
        "the turn is recorded, the message sent, then the companion told: {lines:?}"
    );
    let turn_json = told
        .strip_prefix(&format!("told {} work ", json(&worker)))
        .unwrap_or_else(|| panic!("{told}"));
    let turn: UserTurn = serde_json::from_str(turn_json).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        (turn.id.0, turn.text.as_str()),
        (41, "use the second draft"),
        "the turn the router recorded, with the words"
    );
    assert!(
        desk.log_starting("send ")
            .await
            .contains(r#""kind":"request""#),
        "the person's words go as a request"
    );
    desk.log_has("close s-1").await;
}
