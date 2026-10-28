//! The companion service on a private bus: stand-ins for intentd, companiond, cuad, memoryd and
//! an app own their names; the shell is the connection the intents service would publish on its
//! seat. Everything waits on the service's published state or on a stand-in's record, never on a
//! clock; a test skips, saying so, without a `dbus-daemon`. Never the person's session bus.
//! These are the summon, the ask, the lookups, the journal, the runs and the timeline; the cards,
//! the handles, the openers and what is said to a subagent are `companion_cards_bus.rs`.

use crate::companion_support::{self, desk_or_return};

use std::sync::{Arc, Mutex};

use almanac_core::{
    Fact, FactId, FactState, FactText, FactView, MemoryItem, Settlement, TopicPath, UseCount,
    Validity,
};
use companion_wire::{AnswerPhase, AskWire};
use docket_core::{ContextKeep, Keep, UndoScope, UndoState};
use docket_dbus::{COMPANION_BUS, INTENTS_BUS, PROVIDER_PATH, answer_path};
use docket_vocab::UndoId;
use docket_wire::SummonAnswer;
use ds_core::vocab::MemoryVerb;
use prov::{Actor, InputProof, Label, RunId, SpaceId, UnixSeconds};
use sill_launcher::{Generation, LauncherMode, RunControl, WindowKey};
use sill_model::companion::{
    AskInput, CompanionCommand, CompanionParams, LookupPhase, SightState, Summon, SummonOut,
    SummonParams,
};
use sill_model::units::Ms;
use zbus::zvariant::ObjectPath;

use companion_support::stand_ins::{
    ActsAs, StandInApp, StandInCompanion, StandInControl, StandInManager, StandInMemory,
    StandInRun, StandInRunObject, StandInSearch, StandInSession, StandInWaiting,
};
use companion_support::{app, entry, fast, focus, hit, json, run_path, task, timeline_entry, wire};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_taps_with_no_serving_app_open_the_launcher_in_companion_mode() {
    let mut desk = desk_or_return!();
    desk.send(CompanionCommand::Params(fast()));
    let input = focus(None, None);
    desk.send(CompanionCommand::Tap(input.clone()));
    desk.send(CompanionCommand::Tap(input));
    assert_eq!(
        desk.next_summon().await,
        SummonOut::OpenLauncher {
            mode: LauncherMode::Companion,
            window: Some(WindowKey(4)),
        }
    );
    desk.state(|c| matches!(c.summon, Summon::InLauncher { .. }).then_some(()))
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_serving_app_takes_the_summon_and_the_launcher_stays_shut() {
    let mut desk = desk_or_return!();
    desk.seated().await;
    let mail = desk.peer("org.quire.Mail").await;
    mail.object_server()
        .at(
            PROVIDER_PATH,
            StandInApp {
                answer: Some(SummonAnswer::TookField),
                log: desk.log.clone(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let companion = desk.peer(COMPANION_BUS).await;
    companion
        .object_server()
        .at(
            "/org/quire/Companion1",
            StandInCompanion {
                session: None,
                log: desk.log.clone(),
                view: Arc::default(),
                acts: ActsAs::Accepts,
                routing: None,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    // The app has all the time it needs: this test is about an app that answers.
    desk.send(CompanionCommand::Params(CompanionParams {
        summon: SummonParams {
            app_wait: Ms(60_000),
            ..SummonParams::default()
        },
        ..fast()
    }));
    desk.send(CompanionCommand::Summon(focus(
        Some("org.quire.Mail"),
        None,
    )));
    let serial = desk
        .state(|c| match &c.summon {
            Summon::InApp { serial, .. } => Some(*serial),
            _ => None,
        })
        .await;
    assert_eq!(serial.0, 1);
    let lines = desk.log.lines();
    assert!(
        lines
            .iter()
            .any(|l| l == r#"summon 1 {"kind":"double_tap"}"#),
        "the app was asked with the first serial and the double tap's origin: {lines:?}"
    );
    assert!(
        desk.summons.try_recv().is_err(),
        "the launcher stays shut: the app took the field"
    );
    // The model is warmed once the prompt opens; the stand-in has heard it by now or soon.
    desk.log_has("prepare").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn summon_falls_back_to_launcher_after_app_wait() {
    let mut desk = desk_or_return!();
    desk.seated().await;
    let mail = desk.peer("org.quire.Mail").await;
    mail.object_server()
        .at(
            PROVIDER_PATH,
            StandInApp {
                answer: None,
                log: desk.log.clone(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.send(CompanionCommand::Params(fast()));
    desk.send(CompanionCommand::Summon(focus(
        Some("org.quire.Mail"),
        None,
    )));
    assert_eq!(
        desk.next_summon().await,
        SummonOut::OpenLauncher {
            mode: LauncherMode::Companion,
            window: Some(WindowKey(4)),
        },
        "an app that does not answer in app_wait loses the summon to the launcher"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ask_opens_a_session_records_the_turn_and_follows_the_answer() {
    let mut desk = desk_or_return!();
    let intentd = desk.peer(INTENTS_BUS).await;
    intentd
        .object_server()
        .at("/org/quire/Intents1", StandInSession(desk.log.clone()))
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let view = Arc::new(Mutex::new(json(&wire(AnswerPhase::Thinking, ""))));
    let companiond = desk.peer(COMPANION_BUS).await;
    companiond
        .object_server()
        .at(
            "/org/quire/Companion1",
            StandInCompanion {
                session: None,
                log: desk.log.clone(),
                view: view.clone(),
                acts: ActsAs::Accepts,
                routing: None,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.seated().await;
    desk.send(CompanionCommand::Ask(AskInput {
        prompt: "forward the receipts".to_owned(),
        keep: ContextKeep {
            query: Keep::Dropped,
            results: Keep::Dropped,
            selection: Keep::Dropped,
            window: Keep::Kept,
        },
        window: Some(WindowKey(4)),
        app: Some(app("org.quire.Mail")),
        space: SpaceId::parse("work").unwrap_or_else(|e| panic!("{e}")),
        via: docket_core::TurnVia::Typed,
    }));
    let asked = desk.log_starting("ask ").await;
    let wire_ask: AskWire =
        serde_json::from_str(&asked["ask ".len()..]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(wire_ask.session.as_str(), "s-1");
    assert_eq!(wire_ask.turn.id.0, 41, "the turn the router recorded");
    assert_eq!(wire_ask.turn.text, "forward the receipts");
    assert_eq!(wire_ask.keep.window, Keep::Kept);
    assert_eq!(wire_ask.parent_window.as_str(), "w4");
    assert_eq!(wire_ask.app, Some(app("org.quire.Mail")));
    let turn = desk.log_starting("turn ").await;
    assert!(
        turn.contains("s-1") && turn.contains("forward the receipts"),
        "{turn}"
    );
    // The answer exists thinking; the companion announces it; then it writes and finishes.
    let thinking = desk
        .state(|c| {
            c.answers
                .get(&task())
                .map(|a| (a.prompt.clone(), a.wire.phase.clone()))
        })
        .await;
    assert_eq!(
        thinking,
        ("forward the receipts".to_owned(), AnswerPhase::Thinking)
    );
    let path = answer_path(&task());
    companiond
        .emit_signal(
            None::<&str>,
            "/org/quire/Companion1",
            "org.quire.Companion1",
            "AnswerAdded",
            &(ObjectPath::try_from(path.as_str()).unwrap_or_else(|e| panic!("{e}")),),
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let done = wire(AnswerPhase::Done, "Two receipts found.");
    *view.lock().unwrap_or_else(|e| panic!("{e}")) = json(&done);
    companiond
        .emit_signal(
            None::<&str>,
            path.as_str(),
            "org.quire.Companion1.Answer",
            "Updated",
            &(json(&done),),
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let finished = desk
        .state(|c| {
            c.answers
                .get(&task())
                .filter(|a| a.wire.phase == AnswerPhase::Done)
                .map(|a| a.wire.clone())
        })
        .await;
    assert_eq!(finished, done);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_search_answers_for_its_generation_and_late_hits_extend_it() {
    let mut desk = desk_or_return!();
    let intentd = desk.peer(INTENTS_BUS).await;
    let search = StandInSearch {
        log: desk.log.clone(),
        hits: Arc::new(Mutex::new(vec![hit("a")])),
    };
    intentd
        .object_server()
        .at("/org/quire/Intents1", search)
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let rows = Arc::new(Mutex::new(vec![entry(
        1,
        "Archived 3 threads",
        UndoState::Available,
    )]));
    intentd
        .object_server()
        .at("/org/quire/Intents1", StandInControl(rows))
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.seated().await;
    // The first journal says the service listens to intentd's signals.
    desk.state(|c| (!c.journal.is_empty()).then_some(())).await;
    desk.send(CompanionCommand::Things {
        text: "receipt".to_owned(),
        generation: Generation(9),
    });
    let (phase, found) = desk
        .state(|c| {
            (c.things.generation == Generation(9) && c.things.phase != LookupPhase::Asked)
                .then(|| (c.things.phase, c.things.found.clone()))
        })
        .await;
    assert_eq!(phase, LookupPhase::Answered);
    assert_eq!(found, vec![hit("a")]);
    assert_eq!(desk.log.lines(), vec!["search receipt 9".to_owned()]);
    let shell = desk.shell.unique_name().map(ToString::to_string);
    intentd
        .emit_signal(
            shell.as_deref(),
            "/org/quire/Intents1",
            "org.quire.Intents1.Search",
            "Hits",
            &(9u64, json(&vec![hit("a"), hit("b")])),
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let more = desk
        .state(|c| (c.things.found.len() == 2).then(|| c.things.found.clone()))
        .await;
    assert_eq!(more, vec![hit("a"), hit("b")]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_lookup_without_the_seat_fails_instead_of_waiting() {
    let desk = desk_or_return!();
    desk.send(CompanionCommand::Things {
        text: "receipt".to_owned(),
        generation: Generation(2),
    });
    let phase = desk
        .state(|c| (c.things.generation == Generation(2)).then_some(c.things.phase))
        .await;
    assert!(
        matches!(phase, LookupPhase::Failed | LookupPhase::Asked),
        "{phase:?}"
    );
    let failed = desk
        .state(|c| (c.things.phase == LookupPhase::Failed).then_some(()))
        .await;
    assert_eq!(failed, ());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_journal_is_read_and_follows_the_intentd_signal_and_an_undo_reads_it_again() {
    let mut desk = desk_or_return!();
    let rows = Arc::new(Mutex::new(vec![entry(
        1,
        "Archived 3 threads",
        UndoState::Available,
    )]));
    let intentd = desk.peer(INTENTS_BUS).await;
    intentd
        .object_server()
        .at("/org/quire/Intents1", StandInControl(rows.clone()))
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    intentd
        .object_server()
        .at("/org/quire/Intents1", StandInRun(desk.log.clone()))
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.seated().await;
    let first = desk
        .state(|c| (!c.journal.is_empty()).then(|| c.journal.clone()))
        .await;
    assert_eq!(first.len(), 1);
    // The journal grows behind the shell's back: JournalChanged says so.
    rows.lock()
        .unwrap_or_else(|e| panic!("{e}"))
        .insert(0, entry(2, "Archived a thread", UndoState::Available));
    intentd
        .emit_signal(
            None::<&str>,
            "/org/quire/Intents1",
            "org.quire.Intents1.Control",
            "JournalChanged",
            &(2u64,),
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let two = desk
        .state(|c| (c.journal.len() == 2).then(|| c.journal[0].id))
        .await;
    assert_eq!(two, UndoId(2), "newest first");
    // Undo one row: the stand-in is asked, and the journal is read again.
    rows.lock().unwrap_or_else(|e| panic!("{e}"))[0].state = UndoState::Undone { by: Actor::Cli };
    desk.send(CompanionCommand::Undo(UndoScope::Entry(UndoId(2))));
    desk.log_has("undo 2").await;
    let undone = desk
        .state(|c| matches!(c.journal[0].state, UndoState::Undone { .. }).then_some(()))
        .await;
    assert_eq!(undone, ());
    drop(desk.summons.try_recv());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runs_follow_cuad_and_are_steered_through_their_objects() {
    let mut desk = desk_or_return!();
    let cuad = desk.peer("org.quire.Cua1").await;
    let sight = Arc::new(Mutex::new("No model here can see the screen.".to_owned()));
    cuad.object_server()
        .at(
            "/org/quire/Cua1",
            StandInManager {
                runs: vec![run_path("r_1")],
                sight: sight.clone(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let object = StandInRunObject {
        id: "r-1",
        log: desk.log.clone(),
        state: Arc::new(Mutex::new("acting".to_owned())),
    };
    cuad.object_server()
        .at(run_path("r_1"), object.clone())
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.seated().await;
    let run = RunId::parse("r-1").unwrap_or_else(|e| panic!("{e}"));
    let facts = desk.state(|c| c.runs.get(&run).cloned()).await;
    assert_eq!(facts.goal, "book a table");
    assert_eq!(facts.step, 2);
    assert_eq!(facts.window.as_deref(), Some("Tables"));
    assert_eq!(facts.state, cua_bus::RunStateSlug::Acting);
    assert_eq!(facts.mode, cua_action::RunMode::InPlace);
    // The run says it paused: the facts follow.
    *object.state.lock().unwrap_or_else(|e| panic!("{e}")) = "paused".to_owned();
    cuad.emit_signal(
        None::<&str>,
        run_path("r_1"),
        "org.quire.Cua1.Run",
        "StateChanged",
        &("paused",),
    )
    .await
    .unwrap_or_else(|e| panic!("{e}"));
    desk.state(|c| (c.runs[&run].state == cua_bus::RunStateSlug::Paused).then_some(()))
        .await;
    // A control is the run object's own method.
    desk.send(CompanionCommand::Run {
        run: run.clone(),
        control: RunControl::Stop,
    });
    desk.log_has("cancel r-1").await;
    // The run's object goes away: so do its facts.
    cuad.emit_signal(
        None::<&str>,
        "/org/quire/Cua1",
        "org.quire.Cua1.Manager",
        "RunRemoved",
        &(run_path("r_1"),),
    )
    .await
    .unwrap_or_else(|e| panic!("{e}"));
    desk.state(|c| c.runs.is_empty().then_some(())).await;
    // cuad's note about the screen model reached the companion, and clearing it clears it there.
    desk.state(|c| matches!(c.sight, SightState::CannotSee(_)).then_some(()))
        .await;
    sight.lock().unwrap_or_else(|e| panic!("{e}")).clear();
    let manager = cuad
        .object_server()
        .interface::<_, StandInManager>("/org/quire/Cua1")
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    manager
        .get()
        .await
        .sight_note_changed(manager.signal_emitter())
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.state(|c| (c.sight == SightState::Sees).then_some(()))
        .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_run_is_named_by_its_id_property_and_removed_by_the_path_it_was_added_at() {
    let mut desk = desk_or_return!();
    let cuad = desk.peer("org.quire.Cua1").await;
    // A path that says nothing about the run's id: the object's `Id` is the only name.
    let path = run_path("object_nine");
    cuad.object_server()
        .at(
            "/org/quire/Cua1",
            StandInManager {
                runs: vec![path.clone()],
                ..StandInManager::default()
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    cuad.object_server()
        .at(
            path.clone(),
            StandInRunObject {
                id: "r_9",
                log: desk.log.clone(),
                state: Arc::new(Mutex::new("acting".to_owned())),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.seated().await;
    let run = RunId::parse("r-9").unwrap_or_else(|e| panic!("{e}"));
    let known = desk.state(|c| c.runs.keys().next().cloned()).await;
    assert_eq!(known, run, "the run is the one its Id property names");
    cuad.emit_signal(
        None::<&str>,
        "/org/quire/Cua1",
        "org.quire.Cua1.Manager",
        "RunRemoved",
        &(path,),
    )
    .await
    .unwrap_or_else(|e| panic!("{e}"));
    desk.state(|c| c.runs.is_empty().then_some(())).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_all_pauses_each_run_that_is_going_and_no_other() {
    let mut desk = desk_or_return!();
    let cuad = desk.peer("org.quire.Cua1").await;
    cuad.object_server()
        .at(
            "/org/quire/Cua1",
            StandInManager {
                runs: vec![run_path("r_1"), run_path("r_2")],
                ..StandInManager::default()
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    for (id, state) in [("r_1", "acting"), ("r_2", "paused")] {
        cuad.object_server()
            .at(
                run_path(id),
                StandInRunObject {
                    id,
                    log: desk.log.clone(),
                    state: Arc::new(Mutex::new(state.to_owned())),
                },
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }
    desk.seated().await;
    desk.state(|c| (c.runs.len() == 2).then_some(())).await;
    desk.send(CompanionCommand::StopAll);
    // The run that was going took the call. That the paused one is never called is the pure
    // `going_runs` table's claim (sill-model).
    desk.log_has("pause r_1").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_timeline_is_read_for_the_generation_that_asked() {
    let mut desk = desk_or_return!();
    let memoryd = desk.peer("org.quire.Memory1").await;
    memoryd
        .object_server()
        .at(
            "/org/quire/Memory1",
            StandInMemory(vec![timeline_entry(1), timeline_entry(2)]),
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.seated().await;
    desk.send(CompanionCommand::Timeline {
        generation: Generation(3),
    });
    let rows = desk
        .state(|c| {
            (c.memory.generation == Generation(3) && c.memory.phase == LookupPhase::Answered)
                .then_some(c.memory.found.len())
        })
        .await;
    assert_eq!(rows, 2);
}

fn waiting_fact(last: char) -> FactView {
    FactView {
        fact: Fact {
            id: FactId::parse(&format!("{}{last}", "0".repeat(25)))
                .unwrap_or_else(|e| panic!("{e}")),
            text: FactText::parse("Prefers mornings").unwrap_or_else(|e| panic!("{e}")),
            recorded: UnixSeconds(1),
            by: Actor::Cli,
            label: Label::trusted_user(),
            links: Vec::new(),
            supersedes: Vec::new(),
            valid: Validity::Unstated,
        },
        topic: TopicPath::parse("prefs/meetings").unwrap_or_else(|e| panic!("{e}")),
        state: FactState::Pending,
        sources: Vec::new(),
        used: UseCount(0),
        last_used: None,
        flagged: Vec::new(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn keeping_sends_the_shells_receipt_and_not_keeping_sends_a_discard() {
    let mut desk = desk_or_return!();
    let memoryd = desk.peer("org.quire.Memory1").await;
    let (kept, dropped) = (waiting_fact('1'), waiting_fact('2'));
    memoryd
        .object_server()
        .at(
            "/org/quire/Memory1",
            StandInWaiting {
                facts: vec![kept.clone(), dropped.clone()],
                log: desk.log.clone(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.seated().await;
    desk.send(CompanionCommand::Timeline {
        generation: Generation(1),
    });
    desk.state(|c| {
        (c.waiting.generation == Generation(1) && c.waiting.found.len() == 2).then_some(())
    })
    .await;
    for (view, verb) in [(&kept, MemoryVerb::Keep), (&dropped, MemoryVerb::Discard)] {
        desk.send(CompanionCommand::Memory {
            item: MemoryItem::Fact(view.fact.id.clone()),
            verb,
        });
    }
    let line = |id: &FactId| format!("settle {} ", json(id));
    let keep = desk.log_starting(&line(&kept.fact.id)).await;
    let discard = desk.log_starting(&line(&dropped.fact.id)).await;
    let verdict = |said: String, id: &FactId| {
        let json = said.trim_start_matches(&line(id)).to_owned();
        serde_json::from_str::<Settlement>(&json).unwrap_or_else(|e| panic!("{e}"))
    };
    match verdict(keep, &kept.fact.id) {
        Settlement::Keep(receipt) => {
            assert_eq!(receipt.input, InputProof::ShellCaller);
            assert_eq!(receipt.covers, kept.fact.label.confidentiality);
        }
        Settlement::Discard => panic!("Keep sent a discard"),
    }
    assert_eq!(verdict(discard, &dropped.fact.id), Settlement::Discard);
    desk.state(|c| c.waiting.found.is_empty().then_some(()))
        .await;
}
