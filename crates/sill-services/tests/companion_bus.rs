//! The companion service on a private bus: stand-ins for intentd, companiond, cuad, memoryd and
//! an app own their names; the shell is the connection the intents service would publish on its
//! seat. Everything waits on the service's published state or on a stand-in's record, never on a
//! clock; a test skips, saying so, without a `dbus-daemon`. Never the person's session bus.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use almanac_core::{
    EntryBody, EventRef, KindTag, ReplicaId, Seq, ThingView, TimelineEntry, TimelinePage, UndoRef,
    UserText,
};
use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, AskWire, FooterWire};
use docket_core::{
    ContextKeep, EntityRef, Hit, Keep, SessionOpened, SummonAnswer, UndoEntry, UndoFault, UndoId,
    UndoScope, UndoState,
};
use docket_dbus::{COMPANION_BUS, INTENTS_BUS, PROVIDER_PATH, answer_path};
use porter_core::Count;
use prov::{
    ActionName, Actor, AppName, Effect, EntityId, EntityKey, EntityKind, Label, Labelled, RunId,
    SessionId, SpaceId, TaskId, UnixSeconds,
};
use sill_launcher::{Generation, LauncherMode, RunControl, WindowKey};
use sill_model::companion::{
    AskInput, Busy, Companion, CompanionCommand, CompanionLink, CompanionParams, FocusInput,
    LauncherFacts, LookupPhase, Summon, SummonOut, SummonParams, TapParams,
};
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::seat::{SeatPublisher, seat};
use sill_services::service::{Handle, spawn};
use sill_services::{CompanionLinks, CompanionService};
use sill_testkit::PrivateBus;
use tokio::sync::{Notify, mpsc, watch};
use zbus::fdo;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue};
use zbus::{Connection, interface};

type Details = HashMap<String, OwnedValue>;

/// What a stand-in was asked, in order; a waiter wakes when a line is added.
#[derive(Debug, Clone)]
struct Log(Arc<watch::Sender<Vec<String>>>);

impl Default for Log {
    fn default() -> Log {
        Log(Arc::new(watch::Sender::new(Vec::new())))
    }
}

impl Log {
    fn push(&self, line: impl Into<String>) {
        self.0.send_modify(|lines| lines.push(line.into()));
    }

    fn lines(&self) -> Vec<String> {
        self.0.borrow().clone()
    }

    /// Resolves with what `ready` makes of the lines, once it makes something.
    async fn until<T>(&self, ready: impl Fn(&[String]) -> Option<T>) -> T {
        let mut lines = self.0.subscribe();
        let mut found = None;
        let _ = lines
            .wait_for(|lines| {
                found = ready(lines);
                found.is_some()
            })
            .await;
        found.unwrap_or_else(|| panic!("the log closed"))
    }
}

fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|e| panic!("{e}"))
}

fn app(name: &str) -> AppName {
    AppName::parse(name).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn task() -> TaskId {
    TaskId::parse("t-1").unwrap_or_else(|e| panic!("{e}"))
}

// ---- intentd ----

struct StandInSession(Log);

#[interface(name = "org.quire.Intents1.Session")]
impl StandInSession {
    async fn turn(&self, session: String, turn: String, _options: Details) -> fdo::Result<u64> {
        self.0.push(format!("turn {session} {turn}"));
        Ok(41)
    }
}

#[derive(Clone)]
struct StandInSearch {
    log: Log,
    hits: Arc<Mutex<Vec<Hit>>>,
}

#[interface(name = "org.quire.Intents1.Search")]
impl StandInSearch {
    async fn query(&self, text: String, _scope: String, generation: u64) -> fdo::Result<String> {
        self.log.push(format!("search {text} {generation}"));
        let hits = self.hits.lock().unwrap_or_else(|e| panic!("{e}"));
        Ok(json(&*hits))
    }

    #[zbus(signal)]
    async fn hits(emitter: &SignalEmitter<'_>, generation: u64, hits: &str) -> zbus::Result<()>;
}

struct StandInControl(Arc<Mutex<Vec<UndoEntry>>>);

#[interface(name = "org.quire.Intents1.Control")]
impl StandInControl {
    async fn journal(&self, _filter: String) -> fdo::Result<String> {
        Ok(json(&*self.0.lock().unwrap_or_else(|e| panic!("{e}"))))
    }

    #[zbus(signal)]
    async fn journal_changed(emitter: &SignalEmitter<'_>, rows: u64) -> zbus::Result<()>;
}

struct StandInRun(Log);

#[interface(name = "org.quire.Intents1.Run")]
impl StandInRun {
    async fn undo(
        &self,
        entry: u64,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<OwnedObjectPath> {
        self.0.push(format!("undo {entry}"));
        let path = OwnedObjectPath::try_from("/org/quire/Intents1/request/1")
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let to = header.sender().map(ToString::to_string);
        let (connection, at) = (connection.clone(), path.clone());
        tokio::spawn(async move {
            let end: Result<(), UndoFault> = Ok(());
            let _ = connection
                .emit_signal(
                    to.as_deref(),
                    at.as_ref(),
                    "org.quire.Intents1.Request",
                    "Response",
                    &(0u32, serde_json::to_string(&end).unwrap_or_default()),
                )
                .await;
        });
        Ok(path)
    }
}

// ---- companiond ----

struct StandInCompanion {
    log: Log,
    view: Arc<Mutex<String>>,
    released: Arc<Notify>,
}

#[interface(name = "org.quire.Companion1")]
impl StandInCompanion {
    async fn prepare(&self, _options: Details) {
        self.log.push("prepare");
    }

    async fn open(&self, open: String) -> fdo::Result<String> {
        self.log.push(format!("open {open}"));
        Ok(json(&SessionOpened {
            session: SessionId::parse("s-1").unwrap_or_else(|e| panic!("{e}")),
            task: task(),
        }))
    }

    async fn ask(
        &self,
        ask: String,
        _options: Details,
        #[zbus(object_server)] server: &zbus::ObjectServer,
    ) -> fdo::Result<OwnedObjectPath> {
        self.log.push(format!("ask {ask}"));
        let path = OwnedObjectPath::try_from(answer_path(&task()))
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        server
            .at(
                path.clone(),
                StandInAnswer {
                    log: self.log.clone(),
                    view: self.view.clone(),
                    released: self.released.clone(),
                },
            )
            .await?;
        Ok(path)
    }

    #[zbus(signal)]
    async fn answer_added(emitter: &SignalEmitter<'_>, answer: ObjectPath<'_>) -> zbus::Result<()>;
}

struct StandInAnswer {
    log: Log,
    view: Arc<Mutex<String>>,
    released: Arc<Notify>,
}

#[interface(name = "org.quire.Companion1.Answer")]
impl StandInAnswer {
    async fn act(
        &self,
        action: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<OwnedObjectPath> {
        self.log.push(format!("act {action}"));
        let path = OwnedObjectPath::try_from("/org/quire/Companion1/request/1")
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let to = header.sender().map(ToString::to_string);
        let (connection, at, released) = (connection.clone(), path.clone(), self.released.clone());
        tokio::spawn(async move {
            released.notified().await;
            let _ = connection
                .emit_signal(
                    to.as_deref(),
                    at.as_ref(),
                    "org.quire.Intents1.Request",
                    "Response",
                    &(0u32, "{}".to_owned()),
                )
                .await;
        });
        Ok(path)
    }

    async fn cancel(&self) {
        self.log.push("cancel");
    }

    #[zbus(property)]
    async fn view(&self) -> String {
        self.view.lock().unwrap_or_else(|e| panic!("{e}")).clone()
    }

    #[zbus(signal)]
    async fn updated(emitter: &SignalEmitter<'_>, view: &str) -> zbus::Result<()>;
}

// ---- an app ----

struct StandInApp {
    answer: Option<SummonAnswer>,
    log: Log,
}

#[interface(name = "org.quire.IntentProvider1")]
impl StandInApp {
    async fn summon(&self, serial: u64, origin: String) -> fdo::Result<String> {
        self.log.push(format!("summon {serial} {origin}"));
        match self.answer {
            Some(answer) => Ok(json(&answer)),
            // An app that never answers.
            None => std::future::pending().await,
        }
    }
}

// ---- cuad ----

struct StandInManager {
    runs: Vec<OwnedObjectPath>,
}

#[interface(name = "org.quire.Cua1.Manager")]
impl StandInManager {
    async fn runs(&self) -> Vec<OwnedObjectPath> {
        self.runs.clone()
    }

    #[zbus(signal)]
    async fn run_removed(emitter: &SignalEmitter<'_>, run: OwnedObjectPath) -> zbus::Result<()>;
}

#[derive(Clone)]
struct StandInRunObject {
    id: &'static str,
    log: Log,
    state: Arc<Mutex<String>>,
}

#[interface(name = "org.quire.Cua1.Run")]
impl StandInRunObject {
    async fn pause(&self) {
        self.log.push(format!("pause {}", self.id));
    }

    async fn cancel(&self) {
        self.log.push(format!("cancel {}", self.id));
    }

    #[zbus(property)]
    async fn state(&self) -> String {
        self.state.lock().unwrap_or_else(|e| panic!("{e}")).clone()
    }

    #[zbus(property)]
    async fn step(&self) -> u32 {
        2
    }

    #[zbus(property)]
    async fn goal(&self) -> String {
        "book a table".to_owned()
    }

    #[zbus(property)]
    async fn window(&self) -> String {
        r#"{"app":"org.mozilla.firefox","title":"Tables"}"#.to_owned()
    }

    #[zbus(property)]
    async fn mode(&self) -> String {
        "in_place".to_owned()
    }

    #[zbus(property)]
    async fn budget(&self) -> String {
        String::new()
    }

    #[zbus(signal, name = "StateChanged")]
    async fn phase_changed(emitter: &SignalEmitter<'_>, state: &str) -> zbus::Result<()>;
}

// ---- memoryd ----

struct StandInMemory(Vec<TimelineEntry>);

#[interface(name = "org.quire.Memory1.Control")]
impl StandInMemory {
    async fn timeline(&self, _space: String, _query: String) -> String {
        json(&TimelinePage {
            entries: self.0.clone(),
            next: None,
        })
    }
}

// ---- the desk ----

/// The service under test, its bus, and what the stand-ins hold.
struct Desk {
    bus: PrivateBus,
    companion: Handle<CompanionService>,
    summons: mpsc::UnboundedReceiver<SummonOut>,
    seat: SeatPublisher,
    shell: Connection,
    log: Log,
}

async fn connect(address: &str) -> Connection {
    zbus::connection::Builder::address(address)
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting to the private bus: {e}"))
}

async fn desk() -> Option<Desk> {
    let bus = PrivateBus::start()?;
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(Path::new("/nonexistent/sill-companion-bus"))
    };
    let (seat_publisher, seat_view) = seat();
    let (summons_tx, summons) = mpsc::unbounded_channel();
    let companion = spawn::<CompanionService>(
        &tokio::runtime::Handle::current(),
        &env,
        CompanionLinks {
            seat: seat_view,
            summons: summons_tx,
        },
    );
    let shell = connect(&bus.address).await;
    Some(Desk {
        bus,
        companion,
        summons,
        seat: seat_publisher,
        shell,
        log: Log::default(),
    })
}

impl Desk {
    /// A connection owning `name`.
    async fn peer(&self, name: &str) -> Connection {
        let connection = connect(&self.bus.address).await;
        connection
            .request_name(name)
            .await
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        connection
    }

    /// The shell takes its seat; the service follows.
    async fn seated(&mut self) {
        self.seat.publish(self.shell.clone());
        self.state(|c| matches!(c.link, CompanionLink::Ready).then_some(()))
            .await;
    }

    /// The service's state, once `ready` says yes to it.
    async fn state<T>(&self, ready: impl Fn(&Companion) -> Option<T>) -> T {
        let mut rx = self.companion.state.clone();
        let mut found = None;
        let _ = rx
            .wait_for(|state| {
                found = ready(state);
                found.is_some()
            })
            .await;
        found.unwrap_or_else(|| panic!("the service stopped"))
    }

    fn send(&self, command: CompanionCommand) {
        self.companion
            .control
            .send(command)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// The next effect the service handed the daemon.
    async fn next_summon(&mut self) -> SummonOut {
        self.summons
            .recv()
            .await
            .unwrap_or_else(|| panic!("the service stopped"))
    }
}

fn focus(app_name: Option<&str>, launcher: LauncherFacts) -> FocusInput {
    FocusInput {
        window: Some(WindowKey(4)),
        app: app_name.map(app),
        launcher,
    }
}

fn fast() -> CompanionParams {
    CompanionParams {
        tap: TapParams { window: 60_000 },
        summon: SummonParams {
            app_wait: 1,
            ..SummonParams::default()
        },
    }
}

macro_rules! desk {
    () => {
        match desk().await {
            Some(desk) => desk,
            None => return,
        }
    };
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_taps_with_no_serving_app_open_the_launcher_in_companion_mode() {
    let mut desk = desk!();
    desk.send(CompanionCommand::Params(fast()));
    let input = focus(None, LauncherFacts::Closed);
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
    let mut desk = desk!();
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
                log: desk.log.clone(),
                view: Arc::default(),
                released: Arc::default(),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    // The app has all the time it needs: this test is about an app that answers.
    desk.send(CompanionCommand::Params(CompanionParams {
        summon: SummonParams {
            app_wait: 600_000,
            ..SummonParams::default()
        },
        ..fast()
    }));
    desk.send(CompanionCommand::Summon(focus(
        Some("org.quire.Mail"),
        LauncherFacts::Closed,
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
    let mut desk = desk!();
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
        LauncherFacts::Closed,
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

impl Desk {
    /// Resolves once a stand-in has logged exactly `line`.
    async fn log_has(&self, line: &str) {
        self.log
            .until(|lines| lines.iter().any(|l| l == line).then_some(()))
            .await;
    }

    /// Resolves once the stand-in has logged a line starting with `prefix`; returns it.
    async fn log_starting(&self, prefix: &str) -> String {
        self.log
            .until(|lines| lines.iter().find(|l| l.starts_with(prefix)).cloned())
            .await
    }
}

fn footer() -> FooterWire {
    FooterWire {
        served: Vec::new(),
        sources: Vec::new(),
        keep: ContextKeep {
            query: Keep::Dropped,
            results: Keep::Dropped,
            selection: Keep::Dropped,
            window: Keep::Kept,
        },
    }
}

fn wire(phase: AnswerPhase, text: &str) -> AnswerWire {
    AnswerWire {
        task: task(),
        phase,
        body: AnswerBody::Text {
            lines: vec![docket_core::Reveal::Plain(text.to_owned())],
        },
        footer: footer(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_ask_opens_a_session_records_the_turn_and_follows_the_answer() {
    let mut desk = desk!();
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
                log: desk.log.clone(),
                view: view.clone(),
                released: Arc::default(),
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

fn hit(key: &str) -> Hit {
    Hit {
        entity: EntityRef {
            id: EntityId {
                app: app("org.quire.Mail"),
                kind: EntityKind::parse("mail.thread").unwrap_or_else(|e| panic!("{e}")),
                key: EntityKey::parse(key).unwrap_or_else(|e| panic!("{e}")),
            },
            title: Labelled::new(format!("Receipt {key}"), Label::trusted_user()),
            subtitle: Labelled::new(String::new(), Label::trusted_user()),
        },
        why: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_search_answers_for_its_generation_and_late_hits_extend_it() {
    let mut desk = desk!();
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
    let desk = desk!();
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

fn entry(id: u64, said: &str, state: UndoState) -> UndoEntry {
    UndoEntry {
        id: UndoId(id),
        at: UnixSeconds(1_790_000_000),
        actor: Actor::Cli,
        action: docket_core::ActionRef {
            app: app("org.quire.Mail"),
            name: ActionName::parse("mail.thread.archive").unwrap_or_else(|e| panic!("{e}")),
        },
        said: docket_core::LabelText::parse(said).unwrap_or_else(|e| panic!("{e}")),
        token: docket_core::UndoToken::parse("tok-1").unwrap_or_else(|e| panic!("{e}")),
        run: None,
        session: None,
        state,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_journal_is_read_and_follows_the_intentd_signal_and_an_undo_reads_it_again() {
    let mut desk = desk!();
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
async fn a_card_button_is_busy_until_its_request_has_a_response() {
    let mut desk = desk!();
    let released = Arc::new(Notify::new());
    let view = Arc::new(Mutex::new(json(&wire(AnswerPhase::Done, "ok"))));
    let companiond = desk.peer(COMPANION_BUS).await;
    companiond
        .object_server()
        .at(
            "/org/quire/Companion1",
            StandInCompanion {
                log: desk.log.clone(),
                view: view.clone(),
                released: released.clone(),
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
    // The answer exists (an ask made it), so the card has somewhere to press.
    desk.send(CompanionCommand::Ask(AskInput {
        prompt: "draft a reply".to_owned(),
        keep: ContextKeep {
            query: Keep::Dropped,
            results: Keep::Dropped,
            selection: Keep::Dropped,
            window: Keep::Dropped,
        },
        window: None,
        app: None,
    }));
    desk.state(|c| c.answers.contains_key(&task()).then_some(()))
        .await;
    let action = docket_core::CardActionId::parse("send").unwrap_or_else(|e| panic!("{e}"));
    // The companion announces its answer object so the shell follows it.
    // (The card press does not need the follower: it calls the answer by its path.)
    desk.send(CompanionCommand::Card {
        task: task(),
        action: action.clone(),
    });
    let busy = desk
        .state(|c| match &c.answers[&task()].busy {
            Busy::Acting(id) => Some(id.clone()),
            Busy::Idle => None,
        })
        .await;
    assert_eq!(busy, action);
    desk.log_starting("act ").await;
    released.notify_one();
    desk.state(|c| (c.answers[&task()].busy == Busy::Idle).then_some(()))
        .await;
    assert_eq!(
        desk.log
            .lines()
            .iter()
            .filter(|l| l.starts_with("act "))
            .count(),
        1,
        "one press, one call"
    );
}

fn run_path(id: &str) -> OwnedObjectPath {
    OwnedObjectPath::try_from(format!("/org/quire/Cua1/run/{id}")).unwrap_or_else(|e| panic!("{e}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn runs_follow_cuad_and_are_steered_through_their_objects() {
    let mut desk = desk!();
    let cuad = desk.peer("org.quire.Cua1").await;
    cuad.object_server()
        .at(
            "/org/quire/Cua1",
            StandInManager {
                runs: vec![run_path("r_1")],
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
    assert_eq!(facts.mode, docket_core::RunMode::InPlace);
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
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stop_all_pauses_each_run_that_is_going_and_no_other() {
    let mut desk = desk!();
    let cuad = desk.peer("org.quire.Cua1").await;
    cuad.object_server()
        .at(
            "/org/quire/Cua1",
            StandInManager {
                runs: vec![run_path("r_1"), run_path("r_2")],
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

fn timeline_entry(seq: u64) -> TimelineEntry {
    TimelineEntry {
        event: EventRef {
            space: SpaceId::desktop(),
            replica: ReplicaId([1; 16]),
            seq: Seq(seq),
        },
        occurred: UnixSeconds(1_790_000_000),
        actor: Actor::Cli,
        kind: KindTag::parse("thing.archived").unwrap_or_else(|e| panic!("{e}")),
        effect: Effect::UndoableWrite,
        label: Label::trusted_user(),
        things: vec![ThingView {
            thing: EntityId {
                app: app("org.quire.Mail"),
                kind: EntityKind::parse("mail.thread").unwrap_or_else(|e| panic!("{e}")),
                key: EntityKey::parse("q4").unwrap_or_else(|e| panic!("{e}")),
            },
            title: UserText::new("Q4 budget"),
            subtitle: UserText::new(""),
        }],
        body: EntryBody::Present,
        derived_facts: Count(0),
        undo: UndoRef::None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_timeline_is_read_for_the_generation_that_asked() {
    let mut desk = desk!();
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
