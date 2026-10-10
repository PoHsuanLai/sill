//! An answer names its router session in its `Session` property, and the shell shows the text
//! behind each handle through it: for an answer the shell asked and for one anybody else started,
//! with the session the ask opened as the fallback for an older companion, and a plain reading
//! for a handle the router refuses once the session is closed. On a private bus, with stand-ins
//! for companiond and intentd; every wait is on published state or on a stand-in's record.

use crate::companion_support;

use std::sync::{Arc, Mutex};

use companion_support::stand_ins::{
    ActsAs, MAIL_HANDLE, StandInAnswer, StandInCompanion, StandInOlderSession, StandInSession,
};
use companion_support::{Desk, json, task, wire};
use companion_wire::{AnswerBody, AnswerPhase, AnswerWire};
use docket_core::{ContextKeep, Handle, Keep, Reveal};
use docket_dbus::{COMPANION_BUS, INTENTS_BUS, answer_path};
use prov::SpaceId;
use sill_model::companion::{AskInput, CompanionCommand, answer_line};
use zbus::Connection;
use zbus::zvariant::ObjectPath;

const HANDLE: Handle = Handle(7);

fn with_handle() -> AnswerWire {
    holding(HANDLE)
}

/// An answer whose one line is `handle`.
fn holding(handle: Handle) -> AnswerWire {
    AnswerWire {
        body: AnswerBody::Text {
            lines: vec![Reveal::Handle(handle)],
        },
        ..wire(AnswerPhase::Done, "")
    }
}

struct Bench {
    desk: Desk,
    companiond: Connection,
    view: Arc<Mutex<String>>,
    _intentd: Connection,
}

/// The router the stand-in intentd is.
#[derive(Clone, Copy)]
enum Router {
    /// Knows the labelled reading of a handle.
    Labelled,
    /// Has the plain reading only.
    Older,
}

/// The stand-ins are up and the shell holds its seat. The answers companiond makes name `session`
/// (`None`: an older companion, no such property).
async fn bench(session: Option<&str>) -> Option<Bench> {
    bench_on(session, Router::Labelled, HANDLE).await
}

/// `bench`, on `router`, with an answer that holds `handle`.
async fn bench_on(session: Option<&str>, router: Router, handle: Handle) -> Option<Bench> {
    let mut desk = companion_support::desk().await?;
    let view = Arc::new(Mutex::new(json(&holding(handle))));
    let companiond = desk.peer(COMPANION_BUS).await;
    companiond
        .object_server()
        .at(
            "/org/quire/Companion1",
            StandInCompanion {
                session: session.map(str::to_owned),
                log: desk.log.clone(),
                view: view.clone(),
                acts: ActsAs::Accepts,
                routing: None,
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let intentd = desk.peer(INTENTS_BUS).await;
    let served = intentd.object_server();
    match router {
        Router::Labelled => served
            .at("/org/quire/Intents1", StandInSession(desk.log.clone()))
            .await
            .unwrap_or_else(|e| panic!("{e}")),
        Router::Older => served
            .at("/org/quire/Intents1", StandInOlderSession(desk.log.clone()))
            .await
            .unwrap_or_else(|e| panic!("{e}")),
    };
    desk.seated().await;
    Some(Bench {
        desk,
        companiond,
        view,
        _intentd: intentd,
    })
}

macro_rules! bench {
    ($session:expr) => {
        match bench($session).await {
            Some(bench) => bench,
            None => return,
        }
    };
}

impl Bench {
    /// The shell asks; the stand-in opens session `s-1` and makes the answer.
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

    /// An answer somebody else started appears: the shell never asked for it.
    async fn appears(&self, session: &str) {
        self.companiond
            .object_server()
            .at(
                answer_path(&task()).as_str(),
                StandInAnswer {
                    session: Some(session.to_owned()),
                    log: self.desk.log.clone(),
                    view: self.view.clone(),
                    acts: ActsAs::Accepts,
                    routing: None,
                },
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        // The service subscribes to `AnswerAdded` some moments after it takes its seat, and there
        // is nothing on the bus to wait for: announce until it has followed the answer.
        let mut state = self.desk.companion.state.clone();
        loop {
            self.companiond
                .emit_signal(
                    None::<&str>,
                    "/org/quire/Companion1",
                    "org.quire.Companion1",
                    "AnswerAdded",
                    &(ObjectPath::try_from(answer_path(&task()).as_str())
                        .unwrap_or_else(|e| panic!("{e}")),),
                )
                .await
                .unwrap_or_else(|e| panic!("{e}"));
            let followed = state.wait_for(|c| c.answers.contains_key(&task()));
            if tokio::time::timeout(std::time::Duration::from_millis(200), followed)
                .await
                .is_ok()
            {
                return;
            }
        }
    }

    /// What the answer's first line reads, once the handle has a text.
    async fn line(&self) -> String {
        self.line_of(HANDLE).await
    }

    /// What the answer's first line reads, once `handle` has a text.
    async fn line_of(&self, handle: Handle) -> String {
        self.desk
            .state(|c| {
                let answer = c.answers.get(&task())?;
                answer
                    .shown
                    .contains_key(&handle)
                    .then(|| answer_line(answer).1)
            })
            .await
    }

    fn displays(&self) -> Vec<String> {
        self.desk
            .log
            .lines()
            .into_iter()
            .filter(|l| l.starts_with("display "))
            .collect()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_session_property_is_the_session_a_handle_is_shown_through() {
    let bench = bench!(Some("s-9"));
    bench.ask();
    assert_eq!(bench.line().await, "the text of handle 7");
    assert_eq!(
        bench.displays(),
        vec!["display s-9 7"],
        "the property's session, not the one the ask opened (s-1), once"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_answer_somebody_else_started_shows_its_handle_not_an_ellipsis() {
    let bench = bench!(Some("s-9"));
    bench.appears("s-9").await;
    assert_eq!(bench.line().await, "the text of handle 7");
    assert_eq!(bench.displays(), vec!["display s-9 7"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_older_companion_without_the_property_falls_back_to_the_session_the_ask_opened() {
    let bench = bench!(None);
    bench.ask();
    assert_eq!(bench.line().await, "the text of handle 7");
    assert_eq!(bench.displays(), vec!["display s-1 7"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_handle_refused_after_close_reads_plainly_and_is_not_asked_again() {
    let bench = bench!(Some("s-9"));
    // The answer was dismissed (`Close`) before its handle was first shown.
    bench.desk.log.push("close s-9");
    bench.ask();
    assert_eq!(bench.line().await, "#7", "the handle's plain fallback text");
    // The answer changes again: the refused handle is not asked for in a loop.
    *bench.view.lock().unwrap_or_else(|e| panic!("{e}")) = json(&with_handle());
    bench
        .companiond
        .emit_signal(
            None::<&str>,
            answer_path(&task()).as_str(),
            "org.quire.Companion1.Answer",
            "Updated",
            &(json(&with_handle()),),
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(bench.line().await, "#7");
    assert_eq!(bench.displays(), vec!["display s-9 7"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn somebody_elses_words_are_quoted_and_the_persons_own_are_not() {
    let mail = Handle(MAIL_HANDLE);
    let bench = match bench_on(Some("s-9"), Router::Labelled, mail).await {
        Some(bench) => bench,
        None => return,
    };
    bench.ask();
    assert_eq!(
        bench.line_of(mail).await,
        "\u{201c}the text of handle 9\u{201d}",
        "mail is quoted"
    );
    assert_eq!(bench.displays(), vec!["display s-9 9"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_router_without_labels_is_read_plainly_and_everything_is_quoted() {
    let bench = match bench_on(Some("s-9"), Router::Older, HANDLE).await {
        Some(bench) => bench,
        None => return,
    };
    bench.ask();
    assert_eq!(
        bench.line().await,
        "\u{201c}the text of handle 7\u{201d}",
        "nothing says whose words they are"
    );
    assert_eq!(
        bench.displays(),
        vec!["display s-9 7"],
        "the plain reading, once, after the labelled one was an unknown method"
    );
}
