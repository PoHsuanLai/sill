use std::collections::BTreeMap;

use companion_wire::{
    AnswerBody, AnswerPhase, AnswerWire, CardWire, FooterWire, NeedsYou, PlanStepWire, PlanWire,
    StepWireState,
};
use cua_bus::{CuaOutcome, RunStateSlug};
use docket_core::{
    ActionRef, Args, CallRefusal, CallRequest, CardActionId, ConfirmEnd, ConfirmId, ContextKeep,
    EntityRef, Handle, Hit, Keep, LabelText, Origin, Reveal, RunMode, StepId, SummonAnswer,
    SummonSerial, TargetValue,
};
use ds_core::time::stamp::Stamp;
use ds_core::vocab::CompanionPresence;
use prov::{
    ActionName, AppName, Effect, EntityId, EntityKey, EntityKind, Label, Labelled, RunId,
    SessionId, TaskId,
};
use sill_launcher::{Generation, LauncherMode, WindowKey};

use super::*;

fn task(id: &str) -> TaskId {
    TaskId::parse(id).unwrap_or_else(|e| panic!("{e}"))
}

fn footer() -> FooterWire {
    FooterWire {
        mode: docket_core::Mode::default(),
        served: Vec::new(),
        sources: Vec::new(),
        keep: ContextKeep {
            query: Keep::Kept,
            results: Keep::Dropped,
            selection: Keep::Dropped,
            window: Keep::Dropped,
        },
    }
}

fn wire(id: &str, phase: AnswerPhase) -> AnswerWire {
    AnswerWire {
        task: task(id),
        phase,
        body: AnswerBody::Text {
            lines: vec![Reveal::Plain("Two receipts found.".to_owned())],
        },
        progress: Vec::new(),
        footer: footer(),
    }
}

fn facts(id: &str, phase: AnswerPhase) -> AnswerFacts {
    AnswerFacts {
        prompt: "forward the receipts".to_owned(),
        wire: wire(id, phase),
        busy: Busy::Idle,
        session: None,
        routing: Vec::new(),
        shown: BTreeMap::new(),
    }
}

fn run(id: &str, slug: RunStateSlug, mode: RunMode) -> RunFacts {
    RunFacts {
        run: RunId::parse(id).unwrap_or_else(|e| panic!("{e}")),
        goal: "book a table".to_owned(),
        window: None,
        step: 3,
        state: slug,
        outcome: None,
        mode,
        thought: None,
        budget: None,
    }
}

fn state_with(answers: &[AnswerFacts], runs: &[RunFacts], summon: Summon) -> Companion {
    Companion {
        answers: answers
            .iter()
            .map(|a| (a.wire.task.clone(), a.clone()))
            .collect::<BTreeMap<_, _>>(),
        runs: runs.iter().map(|r| (r.run.clone(), r.clone())).collect(),
        summon,
        ..Companion::default()
    }
}

fn launcher_summon() -> Summon {
    Summon::InLauncher {
        origin: LauncherOrigin::Fresh { window: None },
    }
}

/// Waiting beats acting beats working beats listening beats idle.
#[test]
fn presence_priority_table() {
    use CompanionPresence as P;
    type Case = (&'static str, Companion, CompanionPresence);
    let confirm = AnswerPhase::NeedsYou(NeedsYou::Confirm(
        ConfirmId::parse("c-1").unwrap_or_else(|e| panic!("{e}")),
    ));
    let cases: Vec<Case> = vec![
        ("nothing at all", Companion::default(), P::Idle),
        (
            "a prompt composing",
            state_with(&[], &[], launcher_summon()),
            P::Listening,
        ),
        (
            "an answer thinking",
            state_with(&[facts("t-1", AnswerPhase::Thinking)], &[], Summon::Rest),
            P::Working,
        ),
        (
            "an answer streaming over a prompt composing",
            state_with(
                &[facts("t-1", AnswerPhase::Streaming)],
                &[],
                launcher_summon(),
            ),
            P::Working,
        ),
        (
            "a finished answer is quiet",
            state_with(&[facts("t-1", AnswerPhase::Done)], &[], Summon::Rest),
            P::Idle,
        ),
        (
            "a run aside, running",
            state_with(
                &[],
                &[run("r-1", RunStateSlug::Acting, RunMode::AgentWorkspace)],
                Summon::Rest,
            ),
            P::Working,
        ),
        (
            "a run in place, running",
            state_with(
                &[],
                &[run("r-1", RunStateSlug::Acting, RunMode::InPlace)],
                Summon::Rest,
            ),
            P::Acting,
        ),
        (
            "acting beats working",
            state_with(
                &[facts("t-1", AnswerPhase::Thinking)],
                &[run("r-1", RunStateSlug::Observing, RunMode::InPlace)],
                launcher_summon(),
            ),
            P::Acting,
        ),
        (
            "a confirmation beats everything",
            state_with(
                &[facts("t-1", confirm)],
                &[run("r-1", RunStateSlug::Acting, RunMode::InPlace)],
                launcher_summon(),
            ),
            P::Waiting,
        ),
        (
            "a taken-over run waits on the person",
            state_with(
                &[],
                &[run("r-1", RunStateSlug::TakenOver, RunMode::InPlace)],
                Summon::Rest,
            ),
            P::Waiting,
        ),
        (
            "a run's confirmation waits on the person",
            state_with(
                &[],
                &[run(
                    "r-1",
                    RunStateSlug::Confirming,
                    RunMode::AgentWorkspace,
                )],
                Summon::Rest,
            ),
            P::Waiting,
        ),
        (
            "a finished run is quiet",
            state_with(
                &[],
                &[RunFacts {
                    outcome: Some(CuaOutcome::Done),
                    ..run("r-1", RunStateSlug::Finished, RunMode::InPlace)
                }],
                Summon::Rest,
            ),
            P::Idle,
        ),
    ];
    for (name, state, want) in cases {
        assert_eq!(presence_of(&state), want, "{name}");
    }
}

fn mail() -> AppName {
    AppName::parse("org.quire.Mail").unwrap_or_else(|e| panic!("{e}"))
}

fn focus(launcher: LauncherFacts, serves: Serves) -> FocusFacts {
    FocusFacts {
        window: Some(WindowKey(4)),
        app: Some(mail()),
        launcher,
        serves_companion: serves,
    }
}

fn params() -> CompanionParams {
    CompanionParams::default()
}

#[test]
fn a_tap_pair_summons_and_the_launcher_opens_warmed() {
    let facts = focus(LauncherFacts::Closed, Serves::No);
    let (state, effects) = step(
        Companion::default(),
        CompanionEvent::Tapped(facts.clone()),
        Stamp(100),
        &params(),
    );
    assert_eq!(state.tap, Tap::Armed { until: Stamp(450) });
    assert_eq!(effects, Vec::new(), "one tap summons nothing");
    assert_eq!(wake(&state), Some(Stamp(450)));
    let (state, effects) = step(state, CompanionEvent::Tapped(facts), Stamp(300), &params());
    assert_eq!(state.tap, Tap::Rest);
    assert_eq!(
        effects,
        vec![
            CompanionEffect::Prepare,
            CompanionEffect::Summon(SummonOut::OpenLauncher {
                mode: LauncherMode::Companion,
                window: Some(WindowKey(4)),
            }),
        ]
    );
    assert_eq!(presence_of(&state), CompanionPresence::Listening);
    assert_eq!(wake(&state), None, "a summon in the launcher runs no timer");
}

#[test]
fn a_serving_app_is_asked_with_a_fresh_serial_and_falls_back_when_it_is_silent() {
    let facts = focus(LauncherFacts::Closed, Serves::Yes);
    let (state, effects) = step(
        Companion::default(),
        CompanionEvent::Summoned(facts),
        Stamp(1000),
        &params(),
    );
    let ask = SummonOut::AskApp {
        app: mail(),
        serial: SummonSerial(1),
    };
    assert_eq!(
        effects,
        vec![CompanionEffect::Prepare, CompanionEffect::Summon(ask)]
    );
    assert_eq!(state.asked_serial, 1);
    assert_eq!(
        wake(&state),
        Some(Stamp(1080)),
        "the app has app_wait to answer"
    );
    let (state, effects) = step(state, CompanionEvent::Elapsed, Stamp(1080), &params());
    assert_eq!(
        effects,
        vec![
            CompanionEffect::Prepare,
            CompanionEffect::Summon(SummonOut::OpenLauncher {
                mode: LauncherMode::Companion,
                window: Some(WindowKey(4)),
            }),
        ]
    );
    // The next ask of an app carries the next serial.
    let (state, _) = step(
        Companion {
            summon: Summon::Rest,
            ..state
        },
        CompanionEvent::Summoned(focus(LauncherFacts::Closed, Serves::Yes)),
        Stamp(2000),
        &params(),
    );
    assert_eq!(state.asked_serial, 2);
}

#[test]
fn an_app_that_takes_the_summon_holds_it_until_dismissed() {
    let (state, _) = step(
        Companion::default(),
        CompanionEvent::Summoned(focus(LauncherFacts::Closed, Serves::Yes)),
        Stamp(0),
        &params(),
    );
    let (state, effects) = step(
        state,
        CompanionEvent::AppAnswered(SummonSerial(1), SummonAnswer::TookField),
        Stamp(20),
        &params(),
    );
    assert!(
        matches!(state.summon, Summon::InApp { .. }),
        "{:?}",
        state.summon
    );
    assert_eq!(effects, Vec::new());
    assert_eq!(presence_of(&state), CompanionPresence::Listening);
}

fn hit(key: &str) -> Hit {
    Hit {
        entity: EntityRef {
            id: EntityId {
                app: mail(),
                kind: EntityKind::parse("mail.message").unwrap_or_else(|e| panic!("{e}")),
                key: EntityKey::parse(key).unwrap_or_else(|e| panic!("{e}")),
            },
            title: Labelled::new(format!("Receipt {key}"), Label::trusted_user()),
            subtitle: Labelled::new(String::new(), Label::trusted_user()),
        },
        why: None,
    }
}

#[test]
fn a_lookup_keeps_only_the_newest_ask_and_late_hits_extend_it() {
    let at = Stamp(0);
    let run = |state: Companion, event| step(state, event, at, &params()).0;
    let state = run(
        Companion::default(),
        CompanionEvent::ThingsAsked(Generation(5)),
    );
    assert_eq!(state.things.phase, LookupPhase::Asked);
    // An answer to an older keystroke is dropped.
    let state = run(
        state,
        CompanionEvent::ThingsFound(Generation(4), vec![hit("old")]),
    );
    assert_eq!(state.things.found, Vec::new());
    assert_eq!(state.things.phase, LookupPhase::Asked);
    let state = run(
        state,
        CompanionEvent::ThingsFound(Generation(5), vec![hit("a")]),
    );
    assert_eq!(state.things.phase, LookupPhase::Answered);
    assert_eq!(state.things.found, vec![hit("a")]);
    let state = run(
        state,
        CompanionEvent::ThingsLate(Generation(5), vec![hit("a"), hit("b")]),
    );
    assert_eq!(
        state.things.found,
        vec![hit("a"), hit("b")],
        "a hit already shown is not added again"
    );
    let state = run(state, CompanionEvent::ThingsAsked(Generation(6)));
    assert_eq!(
        (state.things.generation, state.things.found.len()),
        (Generation(6), 0),
        "a newer ask starts the list over"
    );
    let state = run(state, CompanionEvent::ThingsFailed(Generation(6)));
    assert_eq!(state.things.phase, LookupPhase::Failed);
}

fn session() -> SessionId {
    SessionId::parse("s-1").unwrap_or_else(|e| panic!("{e}"))
}

fn send_card() -> CardWire {
    CardWire {
        id: CardActionId::parse("send").unwrap_or_else(|e| panic!("{e}")),
        label: LabelText::parse("Send").unwrap_or_else(|e| panic!("{e}")),
        effect: Effect::Outbound,
        call: CallRequest {
            action: send_action(),
            target: TargetValue::Nothing,
            args: Args::new(),
            origin: Origin::Companion,
        },
    }
}

fn send_action() -> ActionRef {
    ActionRef {
        app: mail(),
        name: ActionName::parse("mail.message.send").unwrap_or_else(|e| panic!("{e}")),
    }
}

/// An answer that offers the send button.
fn draft(phase: AnswerPhase) -> AnswerWire {
    AnswerWire {
        body: AnswerBody::DraftReply {
            to: Vec::new(),
            subject: Reveal::Plain("Re: lunch".to_owned()),
            body: Reveal::Plain("Yes.".to_owned()),
            actions: vec![send_card()],
        },
        ..wire("t-1", phase)
    }
}

/// The plan the companion writes once the send button is pressed, with its step in `state`.
fn sent_plan(phase: AnswerPhase, state: StepWireState) -> AnswerWire {
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
        ..wire("t-1", phase)
    }
}

fn step_of(state: Companion, event: CompanionEvent) -> Companion {
    step(state, event, Stamp(0), &params()).0
}

fn asked_state() -> Companion {
    let state = step_of(
        Companion::default(),
        CompanionEvent::Asked {
            task: task("t-1"),
            prompt: "forward the receipts".to_owned(),
            session: session(),
        },
    );
    step_of(
        state,
        CompanionEvent::Answer(Box::new(draft(AnswerPhase::Done))),
    )
}

#[test]
fn an_answer_keeps_its_prompt_and_its_session_through_updates() {
    let state = asked_state();
    assert_eq!(state.front, Some(task("t-1")));
    let answer = &state.answers[&task("t-1")];
    assert_eq!(answer.prompt, "forward the receipts");
    assert_eq!(answer.session, None, "the object has not named it yet");
    let state = step_of(
        state,
        CompanionEvent::Session {
            task: task("t-1"),
            session: Some(session()),
        },
    );
    let state = step_of(
        state,
        CompanionEvent::Answer(Box::new(wire("t-1", AnswerPhase::Streaming))),
    );
    let answer = &state.answers[&task("t-1")];
    assert_eq!(
        (&answer.prompt, &answer.session, &answer.wire.phase),
        (
            &"forward the receipts".to_owned(),
            &Some(session()),
            &AnswerPhase::Streaming
        )
    );
    let state = step_of(state, CompanionEvent::AnswerGone(task("t-1")));
    assert_eq!((state.answers.len(), state.front), (0, None));
}

#[test]
fn a_card_press_is_busy_until_its_step_ends() {
    let send = CardActionId::parse("send").unwrap_or_else(|e| panic!("{e}"));
    let pressed = step_of(
        asked_state(),
        CompanionEvent::CardPressed {
            task: task("t-1"),
            action: send.clone(),
        },
    );
    let acting = Busy::Acting {
        card: send,
        step: send_action(),
    };
    assert_eq!(pressed.answers[&task("t-1")].busy, acting);
    type Case = (&'static str, AnswerPhase, StepWireState, bool);
    let cases: Vec<Case> = vec![
        (
            "pending",
            AnswerPhase::Streaming,
            StepWireState::Pending,
            true,
        ),
        (
            "running",
            AnswerPhase::Streaming,
            StepWireState::Running,
            true,
        ),
        (
            "done",
            AnswerPhase::Done,
            StepWireState::Done { undo: None },
            false,
        ),
        (
            "failed, with the task fine",
            AnswerPhase::Done,
            StepWireState::Failed(CallRefusal::Unconfirmed(ConfirmEnd::Refused)),
            false,
        ),
        ("skipped", AnswerPhase::Done, StepWireState::Skipped, false),
    ];
    for (name, phase, state, still_busy) in cases {
        let next = step_of(
            pressed.clone(),
            CompanionEvent::Answer(Box::new(sent_plan(phase, state))),
        );
        let want = if still_busy {
            acting.clone()
        } else {
            Busy::Idle
        };
        assert_eq!(next.answers[&task("t-1")].busy, want, "{name}");
    }
    let before_the_plan = step_of(
        pressed.clone(),
        CompanionEvent::Answer(Box::new(draft(AnswerPhase::Streaming))),
    );
    assert_eq!(
        before_the_plan.answers[&task("t-1")].busy,
        acting,
        "an answer that has no plan yet leaves the press going"
    );
    let stopped = step_of(
        pressed,
        CompanionEvent::Answer(Box::new(wire("t-1", AnswerPhase::Cancelled))),
    );
    assert_eq!(
        stopped.answers[&task("t-1")].busy,
        Busy::Idle,
        "a stopped task has nothing left to wait on"
    );
}

#[test]
fn a_button_the_answer_does_not_offer_is_not_pressed_and_a_refusal_is_kept() {
    let ghost = CardActionId::parse("ghost").unwrap_or_else(|e| panic!("{e}"));
    let state = step_of(
        asked_state(),
        CompanionEvent::CardPressed {
            task: task("t-1"),
            action: ghost.clone(),
        },
    );
    assert_eq!(state.answers[&task("t-1")].busy, Busy::Idle);
    let state = step_of(
        state,
        CompanionEvent::CardRefused {
            task: task("t-1"),
            action: ghost.clone(),
        },
    );
    assert_eq!(state.answers[&task("t-1")].busy, Busy::Refused(ghost));
}

#[test]
fn the_answers_phase_follows_needs_you_and_back() {
    let confirm = AnswerPhase::NeedsYou(NeedsYou::Confirm(
        ConfirmId::parse("c-7").unwrap_or_else(|e| panic!("{e}")),
    ));
    let waiting = step_of(
        asked_state(),
        CompanionEvent::Answer(Box::new(sent_plan(confirm, StepWireState::Running))),
    );
    assert_eq!(
        waiting.answers[&task("t-1")].wire.phase,
        AnswerPhase::NeedsYou(NeedsYou::Confirm(
            ConfirmId::parse("c-7").unwrap_or_else(|e| panic!("{e}"))
        ))
    );
    assert_eq!(presence_of(&waiting), CompanionPresence::Waiting);
    assert_eq!(
        answer_line(&waiting.answers[&task("t-1")]).1,
        "Needs your confirmation"
    );
    let done = step_of(
        waiting,
        CompanionEvent::Answer(Box::new(sent_plan(
            AnswerPhase::Done,
            StepWireState::Done { undo: None },
        ))),
    );
    assert_eq!(presence_of(&done), CompanionPresence::Idle);
}

#[test]
fn a_finished_task_whose_step_failed_says_so() {
    let facts = |phase, state, busy| AnswerFacts {
        busy,
        wire: sent_plan(phase, state),
        ..facts("t-1", AnswerPhase::Done)
    };
    let send = CardActionId::parse("send").unwrap_or_else(|e| panic!("{e}"));
    type Case = (&'static str, AnswerFacts, &'static str);
    let cases: Vec<Case> = vec![
        (
            "a failed step is not a done task",
            facts(
                AnswerPhase::Done,
                StepWireState::Failed(CallRefusal::Unconfirmed(ConfirmEnd::Refused)),
                Busy::Idle,
            ),
            "A step failed",
        ),
        (
            "a running step",
            facts(AnswerPhase::Done, StepWireState::Running, Busy::Idle),
            "Working on it",
        ),
        (
            "a press the companion refused",
            facts(
                AnswerPhase::Done,
                StepWireState::Done { undo: None },
                Busy::Refused(send),
            ),
            "Could not do that",
        ),
        (
            "a step that is done",
            facts(
                AnswerPhase::Done,
                StepWireState::Done { undo: None },
                Busy::Idle,
            ),
            "Done",
        ),
    ];
    for (name, answer, want) in cases {
        assert_eq!(answer_line(&answer).1, want, "{name}");
    }
}

#[test]
fn a_handle_waits_for_its_text_and_the_text_is_for_the_screen() {
    let handle = Handle(7);
    let answer = AnswerFacts {
        wire: AnswerWire {
            body: AnswerBody::Text {
                lines: vec![Reveal::Plain("Found".to_owned()), Reveal::Handle(handle)],
            },
            ..wire("t-1", AnswerPhase::Done)
        },
        ..facts("t-1", AnswerPhase::Done)
    };
    assert_eq!(unresolved_handles(&answer), vec![handle]);
    let state = state_with(&[answer], &[], Summon::Rest);
    let shown = step_of(
        state,
        CompanionEvent::Shown {
            task: task("t-1"),
            handle,
            text: "the receipts".to_owned(),
        },
    );
    let answer = &shown.answers[&task("t-1")];
    assert_eq!(
        answer.shown.get(&handle).map(String::as_str),
        Some("the receipts")
    );
    assert_eq!(unresolved_handles(answer), Vec::<Handle>::new());
}

#[test]
fn an_answers_named_session_is_kept_and_a_refused_handle_reads_plainly() {
    let handle = Handle(7);
    let asked = state_with(&[facts("t-1", AnswerPhase::Done)], &[], Summon::Rest);
    assert_eq!(asked.answers[&task("t-1")].session, None);
    let named = SessionId::parse("s-9").unwrap_or_else(|e| panic!("{e}"));
    let kept = step_of(
        asked.clone(),
        CompanionEvent::Session {
            task: task("t-1"),
            session: Some(named.clone()),
        },
    );
    assert_eq!(kept.answers[&task("t-1")].session, Some(named));
    let plain = step_of(
        asked,
        CompanionEvent::Unshown {
            task: task("t-1"),
            handle,
        },
    );
    assert_eq!(
        plain.answers[&task("t-1")]
            .shown
            .get(&handle)
            .map(String::as_str),
        Some("#7")
    );
}

#[test]
fn an_answers_routing_is_kept_through_its_views_and_replaced_when_it_changes() {
    let note: Vec<companion_wire::RouteNote> = serde_json::from_str(
        r#"[{"stage":"answer","served":{"account":"a","model":"claude-haiku-4-5","locality":{"kind":"on_device"}},"why":[],"reached":null}]"#,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let state = state_with(&[facts("t-1", AnswerPhase::Thinking)], &[], Summon::Rest);
    assert_eq!(state.answers[&task("t-1")].routing, Vec::new());
    let routed = step_of(
        state,
        CompanionEvent::Routing {
            task: task("t-1"),
            notes: note.clone(),
        },
    );
    assert_eq!(routed.answers[&task("t-1")].routing, note);
    let viewed = step_of(
        routed.clone(),
        CompanionEvent::Answer(Box::new(wire("t-1", AnswerPhase::Done))),
    );
    assert_eq!(
        viewed.answers[&task("t-1")].routing,
        note,
        "a view keeps it"
    );
    let elsewhere = step_of(
        routed,
        CompanionEvent::Routing {
            task: task("t-9"),
            notes: Vec::new(),
        },
    );
    assert_eq!(
        elsewhere.answers[&task("t-1")].routing,
        note,
        "another answer's notes are not this one's"
    );
}

#[test]
fn an_answer_from_elsewhere_is_kept_without_a_prompt() {
    let (state, _) = step(
        Companion::default(),
        CompanionEvent::Answer(Box::new(wire("t-9", AnswerPhase::Done))),
        Stamp(0),
        &params(),
    );
    assert_eq!(state.answers[&task("t-9")].prompt, "");
    assert_eq!(state.front, Some(task("t-9")));
}

#[test]
fn runs_come_and_go_by_their_id() {
    let at = Stamp(0);
    let first = run("r-1", RunStateSlug::Acting, RunMode::InPlace);
    let (state, _) = step(
        Companion::default(),
        CompanionEvent::Run(Box::new(first.clone())),
        at,
        &params(),
    );
    let moved = RunFacts {
        step: 4,
        ..first.clone()
    };
    let (state, _) = step(
        state,
        CompanionEvent::Run(Box::new(moved.clone())),
        at,
        &params(),
    );
    assert_eq!(state.runs.len(), 1);
    assert_eq!(state.runs[&first.run], moved);
    let (state, _) = step(state, CompanionEvent::RunGone(first.run), at, &params());
    assert!(state.runs.is_empty());
}

#[test]
fn the_screen_model_note_sets_and_clears_what_the_companion_says() {
    let said = |state, note: &str| {
        step(
            state,
            CompanionEvent::Sight(SightState::of(note)),
            Stamp(0),
            &params(),
        )
        .0
    };
    let blind = said(Companion::default(), "No model here can see the screen.");
    assert_eq!(
        blind.sight,
        SightState::CannotSee(
            NoteText::new("No model here can see the screen.").unwrap_or_else(|| panic!("empty"))
        )
    );
    assert_eq!(said(blind, "").sight, SightState::Sees);
    assert_eq!(NoteText::new("   "), None);
}

#[test]
fn stop_all_pauses_the_runs_that_are_going_and_no_other() {
    let finished = RunFacts {
        outcome: Some(CuaOutcome::Done),
        ..run("r-4", RunStateSlug::Finished, RunMode::InPlace)
    };
    let state = state_with(
        &[],
        &[
            run("r-1", RunStateSlug::Acting, RunMode::InPlace),
            run("r-2", RunStateSlug::Paused, RunMode::InPlace),
            run("r-3", RunStateSlug::Confirming, RunMode::AgentWorkspace),
            finished,
        ],
        Summon::Rest,
    );
    let ids: Vec<String> = going_runs(&state)
        .iter()
        .map(|r| r.as_str().to_owned())
        .collect();
    assert_eq!(
        ids,
        ["r-1", "r-3"],
        "paused and finished runs are left alone"
    );
}

#[test]
fn a_rows_second_line_is_the_answer_by_phase_in_plain_words() {
    const CHECKING: &str = "I'll check my memory to see what I've remembered.";
    const ANSWER: &str = "The capital of Taiwan is **Taipei**. Remember it?";
    let said = |phase: AnswerPhase, lines: &[&str]| AnswerFacts {
        wire: AnswerWire {
            body: AnswerBody::Text {
                lines: lines
                    .iter()
                    .map(|line| Reveal::Plain((*line).to_owned()))
                    .collect(),
            },
            ..wire("t-1", phase)
        },
        ..facts("t-1", AnswerPhase::Done)
    };
    let question = AnswerPhase::NeedsYou(NeedsYou::Question {
        text: "Remember *this*?".to_owned(),
        choices: Vec::new(),
    });
    type Case = (&'static str, AnswerFacts, &'static str);
    let cases: Vec<Case> = vec![
        (
            "done: the last line, not the first",
            said(AnswerPhase::Done, &[CHECKING, "", ANSWER]),
            "The capital of Taiwan is Taipei. Remember it?",
        ),
        (
            "done with one line",
            said(AnswerPhase::Done, &["`ls` lists files"]),
            "ls lists files",
        ),
        ("done with no line", said(AnswerPhase::Done, &[]), "Done"),
        (
            "running: the latest progress",
            said(AnswerPhase::Streaming, &[CHECKING, "Looking it up"]),
            "Looking it up",
        ),
        (
            "running with none yet",
            said(AnswerPhase::Streaming, &[]),
            "Writing",
        ),
        (
            "needs you: the question",
            said(question, &[CHECKING, ANSWER]),
            "Remember this?",
        ),
        (
            "failed keeps its words",
            said(AnswerPhase::Failed, &[ANSWER]),
            "Could not finish",
        ),
    ];
    for (name, answer, want) in cases {
        assert_eq!(answer_line(&answer).1, want, "{name}");
    }
}
