//! An answer says how it was reached in its `Routing` property, and the shell keeps the notes
//! with the answer: read as the answer appears, read again when the property changes, and
//! nothing for `[]` or for an older companion without the property. The words the footer shows
//! are `companion_wire::footer_line`'s. A named model that cannot serve fails the task with a
//! readable reason, which the answer's view carries. On a private bus, with stand-ins for
//! companiond and intentd.

use crate::companion_support;

use std::sync::{Arc, Mutex};

use companion_support::stand_ins::{ActsAs, StandInAnswer, StandInCompanion, StandInSession};
use companion_support::{Desk, haiku_notes, json, route_notes, task, wire};
use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, RefusalWire, RouteNote, footer_line};
use docket_core::{ContextKeep, Keep};
use docket_dbus::{COMPANION_BUS, INTENTS_BUS, answer_path};
use prov::SpaceId;
use sill_model::companion::{AskInput, CompanionCommand};
use zbus::Connection;

const HAIKU_LINE: &str = "Answered by Claude Haiku 4.5 via OpenRouter (already loaded)";

struct Bench {
    desk: Desk,
    companiond: Connection,
    view: Arc<Mutex<String>>,
    routing: Arc<Mutex<String>>,
    _intentd: Connection,
}

/// The stand-ins are up and the shell holds its seat. The answers companiond makes have a
/// `Routing` property that starts as `first` (`None`: an older companion, no such property).
async fn bench(first: Option<&str>) -> Option<Bench> {
    let mut desk = companion_support::desk().await?;
    let view = Arc::new(Mutex::new(json(&wire(AnswerPhase::Thinking, "first"))));
    let routing = Arc::new(Mutex::new(first.unwrap_or_default().to_owned()));
    let companiond = desk.peer(COMPANION_BUS).await;
    companiond
        .object_server()
        .at(
            "/org/quire/Companion1",
            StandInCompanion {
                session: Some("s-1".to_owned()),
                log: desk.log.clone(),
                view: view.clone(),
                acts: ActsAs::Accepts,
                routing: first.map(|_| routing.clone()),
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
    Some(Bench {
        desk,
        companiond,
        view,
        routing,
        _intentd: intentd,
    })
}

macro_rules! bench {
    ($routing:expr) => {
        match bench($routing).await {
            Some(bench) => bench,
            None => return,
        }
    };
}

impl Bench {
    fn ask(&self) {
        self.desk.send(CompanionCommand::Ask(AskInput {
            prompt: "find the receipts".to_owned(),
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

    /// The answer's view becomes `wire`, and the object says so.
    async fn views(&self, wire: &AnswerWire) {
        *self.view.lock().unwrap_or_else(|e| panic!("{e}")) = json(wire);
        self.companiond
            .emit_signal(
                None::<&str>,
                answer_path(&task()).as_str(),
                "org.quire.Companion1.Answer",
                "Updated",
                &(json(wire),),
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// The answer's `Routing` becomes `text`, and the object says so.
    async fn routes(&self, text: &str) {
        text.clone_into(&mut self.routing.lock().unwrap_or_else(|e| panic!("{e}")));
        let path = answer_path(&task());
        let object = self
            .companiond
            .object_server()
            .interface::<_, StandInAnswer>(path.as_str())
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        object
            .get()
            .await
            .routing_changed(object.signal_emitter())
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// The notes the shell holds for the answer, once its view reads `text` (a later view than
    /// the notes were read with, so a note the shell was ever going to read is there).
    async fn notes_after_view(&self, text: &str) -> Vec<RouteNote> {
        self.views(&wire(AnswerPhase::Done, text)).await;
        let want = AnswerBody::Text {
            lines: vec![docket_core::Reveal::Plain(text.to_owned())],
        };
        self.desk
            .state(|c| {
                let answer = c.answers.get(&task())?;
                (answer.wire.body == want).then(|| answer.routing.clone())
            })
            .await
    }

    async fn routing_is(&self, want: Vec<RouteNote>) {
        self.desk
            .state(|c| (c.answers.get(&task())?.routing == want).then_some(()))
            .await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_routing_read_as_the_answer_appears_prints_the_footer_line() {
    let bench = bench!(Some(&json(&haiku_notes())));
    bench.ask();
    bench.routing_is(haiku_notes()).await;
    assert_eq!(footer_line(&haiku_notes()), HAIKU_LINE);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_footer_updates_when_the_routing_changes() {
    let bench = bench!(Some("[]"));
    bench.ask();
    assert_eq!(bench.notes_after_view("one").await, Vec::new());
    bench.routes(&json(&haiku_notes())).await;
    bench.routing_is(haiku_notes()).await;
    let heard = route_notes(
        r#"[{"stage":"hear","served":{"account":"local","model":"whisper","locality":{"kind":"on_device"}},"why":[],"reached":null}]"#,
    );
    let both: Vec<RouteNote> = heard.iter().chain(&haiku_notes()).cloned().collect();
    bench.routes(&json(&both)).await;
    bench.routing_is(both.clone()).await;
    assert_eq!(
        footer_line(&both),
        format!("Heard by Whisper · {HAIKU_LINE}")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn no_notes_means_no_footer() {
    let bench = bench!(Some("[]"));
    bench.ask();
    assert_eq!(bench.notes_after_view("later").await, Vec::new());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_older_companion_without_the_property_has_no_footer() {
    let bench = bench!(None);
    bench.ask();
    assert_eq!(bench.notes_after_view("later").await, Vec::new());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_declined_named_model_fails_the_task_with_its_reason() {
    let bench = bench!(Some("[]"));
    bench.ask();
    let why = "Kimi K2.6 cannot answer: it is not installed on this computer.";
    let failed = AnswerWire {
        phase: AnswerPhase::Failed,
        body: AnswerBody::Refused(RefusalWire::Failed(why.to_owned())),
        ..wire(AnswerPhase::Failed, "")
    };
    bench.views(&failed).await;
    let kept = bench
        .desk
        .state(|c| {
            let answer = c.answers.get(&task())?;
            matches!(answer.wire.phase, AnswerPhase::Failed).then(|| answer.wire.body.clone())
        })
        .await;
    assert_eq!(
        kept,
        AnswerBody::Refused(RefusalWire::Failed(why.to_owned()))
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_turn_that_meets_no_model_opens_nothing_by_itself() {
    let mut bench = bench!(Some("[]"));
    bench.ask();
    let refused = AnswerWire {
        phase: AnswerPhase::Failed,
        body: AnswerBody::Refused(RefusalWire::NoModel),
        ..wire(AnswerPhase::Failed, "")
    };
    bench.views(&refused).await;
    // By the time the later view is held, the service has handled the refusal, and it asked
    // for nothing.
    bench.notes_after_view("later").await;
    assert!(
        bench.desk.launched.try_recv().is_err(),
        "a refusal opens nothing until the person presses the button"
    );
}
