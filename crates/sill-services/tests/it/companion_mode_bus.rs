//! The conversation's mode over the bus: the box learns it from the session an ask opened,
//! switches it with `Session.SetMode`, follows the router's `ModeChanged`, carries it on the next
//! turn, and goes locked when the router refuses a switch. On a private bus, with stand-ins for
//! companiond and intentd; every wait is on published state or on a stand-in's record.

use crate::companion_support;

use std::sync::{Arc, Mutex};

use companion_support::stand_ins::{ActsAs, StandInCompanion, StandInModeSession, SwitchAnswer};
use companion_support::{Desk, json, wire};
use companion_wire::AnswerPhase;
use docket_core::{ContextKeep, Keep};
use docket_dbus::{COMPANION_BUS, INTENTS_BUS};
use docket_permissions_core::Mode;
use prov::SpaceId;
use sill_model::companion::{AskInput, CompanionCommand, Knowing, ModeLock, ModeMsg, mode_view};
use zbus::Connection;

struct Bench {
    desk: Desk,
    answer: Arc<Mutex<SwitchAnswer>>,
    _companiond: Connection,
    _intentd: Connection,
}

async fn bench() -> Option<Bench> {
    let mut desk = companion_support::desk().await?;
    let view = Arc::new(Mutex::new(json(&wire(AnswerPhase::Done, "Done."))));
    let companiond = desk.peer(COMPANION_BUS).await;
    companiond
        .object_server()
        .at(
            "/org/quire/Companion1",
            StandInCompanion {
                session: Some("s-1".to_owned()),
                log: desk.log.clone(),
                view,
                acts: ActsAs::Accepts,
                routing: None,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let answer = Arc::new(Mutex::new(SwitchAnswer::Done));
    let intentd = desk.peer(INTENTS_BUS).await;
    intentd
        .object_server()
        .at(
            "/org/quire/Intents1",
            StandInModeSession {
                log: desk.log.clone(),
                answer: answer.clone(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    desk.seated().await;
    Some(Bench {
        desk,
        answer,
        _companiond: companiond,
        _intentd: intentd,
    })
}

impl Bench {
    fn ask(&self) {
        self.desk.send(CompanionCommand::Ask(AskInput {
            prompt: "tidy the inbox".to_owned(),
            keep: ContextKeep {
                query: Keep::Dropped,
                results: Keep::Dropped,
                selection: Keep::Dropped,
                window: Keep::Dropped,
            },
            window: None,
            app: None,
            space: SpaceId::desktop(),
            via: docket_core::TurnVia::Typed,
        }));
    }

    fn mode(&self, msg: ModeMsg) {
        self.desk.send(CompanionCommand::Mode(msg));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_switch_reaches_the_router_and_the_next_turn_carries_it() {
    let Some(bench) = bench().await else { return };
    bench.ask();
    let opened = bench
        .desk
        .state(|c| (c.mode.knowing == Knowing::Known).then(|| c.mode.clone()))
        .await;
    assert_eq!(opened.mode, Mode::Auto, "the session opened in Auto");
    let first = bench.desk.log_starting("turn ").await;
    assert!(
        !first.contains("\"mode\""),
        "the first turn is sent before the router has said a mode: {first}"
    );
    bench.mode(ModeMsg::Switch(Mode::Manual));
    bench.desk.log_has("set_mode s-1 manual").await;
    bench
        .desk
        .state(|c| (mode_view(&c.mode).mode == Mode::Manual).then_some(()))
        .await;
    bench.ask();
    let second = bench
        .desk
        .log
        .until(|lines| {
            let turns: Vec<&String> = lines.iter().filter(|l| l.starts_with("turn ")).collect();
            (turns.len() == 2).then(|| turns[1].clone())
        })
        .await;
    assert!(
        second.contains("\"mode\":\"manual\""),
        "the second turn carries the mode: {second}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_refused_switch_locks_the_box_in_manual() {
    let Some(bench) = bench().await else { return };
    bench.ask();
    bench
        .desk
        .state(|c| (c.mode.knowing == Knowing::Known).then_some(()))
        .await;
    *bench.answer.lock().unwrap_or_else(|e| panic!("{e}")) = SwitchAnswer::Locked;
    bench.mode(ModeMsg::Flip);
    let locked = bench
        .desk
        .state(|c| (c.mode.lock == ModeLock::Locked).then(|| mode_view(&c.mode)))
        .await;
    assert_eq!(locked.mode, Mode::Manual);
    bench.mode(ModeMsg::Flip);
    // The ask follows the flip on the same channel: its turn proves the flip was handled.
    bench.ask();
    bench
        .desk
        .log
        .until(|lines| (lines.iter().filter(|l| l.starts_with("turn ")).count() == 2).then_some(()))
        .await;
    let asked = bench
        .desk
        .log
        .lines()
        .iter()
        .filter(|l| l.starts_with("set_mode "))
        .count();
    assert_eq!(asked, 1, "a locked box asks the router nothing more");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_probe_switches_any_conversation_and_only_a_change_of_the_boxs_own_reaches_it() {
    let Some(bench) = bench().await else { return };
    bench.ask();
    bench
        .desk
        .state(|c| (c.mode.knowing == Knowing::Known).then_some(()))
        .await;
    let probe = |session: &str, mode| CompanionCommand::SessionMode {
        session: prov::SessionId::parse(session).unwrap_or_else(|e| panic!("{e}")),
        mode,
    };
    // Another conversation, refused: the router is asked, the box's own stays as it was.
    *bench.answer.lock().unwrap_or_else(|e| panic!("{e}")) = SwitchAnswer::Locked;
    bench.desk.send(probe("s-9", Mode::Manual));
    bench.desk.log_has("set_mode s-9 manual").await;
    // The ask follows the probe on the same channel: its turn proves the probe was handled.
    bench.ask();
    bench
        .desk
        .log
        .until(|lines| (lines.iter().filter(|l| l.starts_with("turn ")).count() == 2).then_some(()))
        .await;
    let unmoved = bench.desk.state(|c| Some(mode_view(&c.mode))).await;
    assert_eq!(
        (unmoved.mode, unmoved.lock),
        (Mode::Auto, ModeLock::Free),
        "a refusal for another conversation says nothing of the box's"
    );
    // The box's own conversation, done: the change reaches the box.
    *bench.answer.lock().unwrap_or_else(|e| panic!("{e}")) = SwitchAnswer::Done;
    bench.desk.send(probe("s-1", Mode::Manual));
    bench.desk.log_has("set_mode s-1 manual").await;
    bench
        .desk
        .state(|c| (mode_view(&c.mode).mode == Mode::Manual).then_some(()))
        .await;
}
