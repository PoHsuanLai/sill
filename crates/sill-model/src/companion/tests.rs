use std::collections::BTreeMap;

use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, FooterWire, NeedsYou};
use cua_bus::{CuaOutcome, RunStateSlug};
use docket_core::{
    ConfirmId, ContextKeep, EntityRef, Hit, Keep, Reveal, RunMode, SummonAnswer, SummonSerial,
};
use ds_core::time::stamp::Stamp;
use ds_core::vocab::CompanionPresence;
use prov::{AppName, EntityId, EntityKey, EntityKind, Label, Labelled, RunId, TaskId};
use sill_launcher::{Generation, LauncherMode, WindowKey};

use super::*;

fn task(id: &str) -> TaskId {
    TaskId::parse(id).unwrap_or_else(|e| panic!("{e}"))
}

fn footer() -> FooterWire {
    FooterWire {
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
        footer: footer(),
    }
}

fn facts(id: &str, phase: AnswerPhase) -> AnswerFacts {
    AnswerFacts {
        prompt: "forward the receipts".to_owned(),
        wire: wire(id, phase),
        busy: Busy::Idle,
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

#[test]
fn an_answer_keeps_its_prompt_and_its_button_state_through_updates() {
    let at = Stamp(0);
    let run = |state: Companion, event| step(state, event, at, &params()).0;
    let state = run(
        Companion::default(),
        CompanionEvent::Asked {
            task: task("t-1"),
            prompt: "forward the receipts".to_owned(),
        },
    );
    assert_eq!(state.front, Some(task("t-1")));
    assert_eq!(
        state.answers[&task("t-1")].wire.phase,
        AnswerPhase::Thinking,
        "asked, nothing written yet"
    );
    let action = docket_core::CardActionId::parse("send").unwrap_or_else(|e| panic!("{e}"));
    let state = run(
        state,
        CompanionEvent::CardPressed {
            task: task("t-1"),
            action: action.clone(),
        },
    );
    let state = run(
        state,
        CompanionEvent::Answer(wire("t-1", AnswerPhase::Streaming)),
    );
    let answer = &state.answers[&task("t-1")];
    assert_eq!(answer.prompt, "forward the receipts");
    assert_eq!(answer.busy, Busy::Acting(action));
    assert_eq!(answer.wire.phase, AnswerPhase::Streaming);
    let state = run(state, CompanionEvent::CardEnded(task("t-1")));
    assert_eq!(state.answers[&task("t-1")].busy, Busy::Idle);
    let state = run(state, CompanionEvent::AnswerGone(task("t-1")));
    assert_eq!((state.answers.len(), state.front), (0, None));
}

#[test]
fn an_answer_from_elsewhere_is_kept_without_a_prompt() {
    let (state, _) = step(
        Companion::default(),
        CompanionEvent::Answer(wire("t-9", AnswerPhase::Done)),
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
