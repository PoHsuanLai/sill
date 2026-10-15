//! The companion service's test bench: a private bus, the stand-ins that own the peers' names
//! (`stand_ins`), and the desk that holds the service under test and the connection the shell's
//! seat would publish. Everything waits on the service's published state or on a stand-in's
//! record, never on a clock.

pub mod stand_ins;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use almanac_core::{
    EntryBody, EventRef, KindTag, ReplicaId, Seq, ThingView, TimelineEntry, UndoRef, UserText,
};
use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, FooterWire};
use docket_core::{ContextKeep, EntityRef, Hit, Keep, UndoEntry, UndoId, UndoState};
use porter_core::Count;
use prov::{
    ActionName, Actor, AppName, Effect, EntityId, EntityKey, EntityKind, Label, Labelled, SpaceId,
    TaskId, UnixSeconds,
};
use sill_launcher::WindowKey;
use sill_model::companion::{
    Companion, CompanionCommand, CompanionLink, CompanionParams, FocusInput, LauncherFacts,
    SummonOut, SummonParams, TapParams,
};
use sill_model::launch::LaunchCommand;
use sill_model::units::Ms;
use sill_services::control::Control;
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::seat::{SeatPublisher, seat};
use sill_services::service::{Handle, spawn};
use sill_services::{CompanionLinks, CompanionService};
use sill_testkit::PrivateBus;
use tokio::sync::{mpsc, watch};
use zbus::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

/// How long a wait for an event may take before the test fails instead of hanging: a failure
/// bound far above any real delivery, not a delay anything depends on.
const GIVE_UP: std::time::Duration = std::time::Duration::from_secs(60);

pub type Details = HashMap<String, OwnedValue>;

/// What a stand-in was asked, in order; a waiter wakes when a line is added.
#[derive(Debug, Clone)]
pub struct Log(pub Arc<watch::Sender<Vec<String>>>);

impl Default for Log {
    fn default() -> Log {
        Log(Arc::new(watch::Sender::new(Vec::new())))
    }
}

impl Log {
    pub fn push(&self, line: impl Into<String>) {
        self.0.send_modify(|lines| lines.push(line.into()));
    }

    pub fn lines(&self) -> Vec<String> {
        self.0.borrow().clone()
    }

    /// Resolves with what `ready` makes of the lines, once it makes something.
    pub async fn until<T>(&self, ready: impl Fn(&[String]) -> Option<T>) -> T {
        let mut lines = self.0.subscribe();
        let mut found = None;
        let waited = tokio::time::timeout(
            GIVE_UP,
            lines.wait_for(|lines| {
                found = ready(lines);
                found.is_some()
            }),
        )
        .await;
        match (found, waited) {
            (Some(found), _) => found,
            (None, Ok(_)) => panic!("the log closed"),
            (None, Err(_)) => panic!(
                "no stand-in recorded what the test waits for; the record is {:?}",
                self.lines()
            ),
        }
    }
}

pub fn json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|e| panic!("{e}"))
}

pub fn app(name: &str) -> AppName {
    AppName::parse(name).unwrap_or_else(|e| panic!("{name}: {e}"))
}

pub fn task() -> TaskId {
    TaskId::parse("t-1").unwrap_or_else(|e| panic!("{e}"))
}

// ---- the desk ----

/// The service under test, its bus, and what the stand-ins hold.
pub struct Desk {
    pub bus: PrivateBus,
    pub companion: Handle<CompanionService>,
    pub summons: mpsc::UnboundedReceiver<SummonOut>,
    /// What the service asked the launch service to do.
    pub launched: mpsc::UnboundedReceiver<LaunchCommand>,
    pub seat: SeatPublisher,
    pub shell: Connection,
    pub log: Log,
}

pub async fn connect(address: &str) -> Connection {
    zbus::connection::Builder::address(address)
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting to the private bus: {e}"))
}

pub async fn desk() -> Option<Desk> {
    let bus = PrivateBus::start()?;
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(Path::new("/nonexistent/sill-companion-bus"))
    };
    let (seat_publisher, seat_view) = seat();
    let (summons_tx, summons) = mpsc::unbounded_channel();
    let (launch, launched) = Control::channel();
    let companion = spawn::<CompanionService>(
        &tokio::runtime::Handle::current(),
        &env,
        CompanionLinks {
            seat: seat_view,
            summons: summons_tx,
            launch,
        },
    );
    let shell = connect(&bus.address).await;
    Some(Desk {
        bus,
        companion,
        summons,
        launched,
        seat: seat_publisher,
        shell,
        log: Log::default(),
    })
}

impl Desk {
    /// A connection owning `name`.
    pub async fn peer(&self, name: &str) -> Connection {
        let connection = connect(&self.bus.address).await;
        connection
            .request_name(name)
            .await
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        connection
    }

    /// The shell takes its seat; the service follows.
    pub async fn seated(&mut self) {
        self.seat.publish(self.shell.clone());
        self.state(|c| matches!(c.link, CompanionLink::Ready).then_some(()))
            .await;
    }

    /// The service's state, once `ready` says yes to it.
    pub async fn state<T>(&self, ready: impl Fn(&Companion) -> Option<T>) -> T {
        let mut rx = self.companion.state.clone();
        let mut found = None;
        let waited = tokio::time::timeout(
            GIVE_UP,
            rx.wait_for(|state| {
                found = ready(state);
                found.is_some()
            }),
        )
        .await;
        match (found, waited) {
            (Some(found), _) => found,
            (None, Ok(_)) => panic!("the service stopped"),
            (None, Err(_)) => panic!(
                "the service never published what the test waits for; it holds {:?}; the record is {:?}",
                self.companion.state.borrow(),
                self.log.lines()
            ),
        }
    }

    pub fn send(&self, command: CompanionCommand) {
        self.companion
            .control
            .send(command)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// The next thing the service asked the launch service to do.
    pub async fn next_launched(&mut self) -> LaunchCommand {
        tokio::time::timeout(GIVE_UP, self.launched.recv())
            .await
            .unwrap_or_else(|_| panic!("the service asked the launch service for nothing"))
            .unwrap_or_else(|| panic!("the service stopped"))
    }

    /// The next effect the service handed the daemon.
    pub async fn next_summon(&mut self) -> SummonOut {
        self.summons
            .recv()
            .await
            .unwrap_or_else(|| panic!("the service stopped"))
    }
}

pub fn focus(app_name: Option<&str>, launcher: LauncherFacts) -> FocusInput {
    FocusInput {
        window: Some(WindowKey(4)),
        app: app_name.map(app),
        launcher,
    }
}

pub fn fast() -> CompanionParams {
    CompanionParams {
        tap: TapParams { window: Ms(60_000) },
        summon: SummonParams {
            app_wait: Ms(1),
            ..SummonParams::default()
        },
    }
}

macro_rules! desk_or_return {
    () => {
        match $crate::companion_support::desk().await {
            Some(desk) => desk,
            None => return,
        }
    };
}

pub(crate) use desk_or_return;

impl Desk {
    /// Resolves once a stand-in has logged exactly `line`.
    pub async fn log_has(&self, line: &str) {
        self.log
            .until(|lines| lines.iter().any(|l| l == line).then_some(()))
            .await;
    }

    /// Resolves once the stand-in has logged a line starting with `prefix`; returns it.
    pub async fn log_starting(&self, prefix: &str) -> String {
        self.log
            .until(|lines| lines.iter().find(|l| l.starts_with(prefix)).cloned())
            .await
    }
}

pub fn footer() -> FooterWire {
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

pub fn wire(phase: AnswerPhase, text: &str) -> AnswerWire {
    AnswerWire {
        task: task(),
        phase,
        body: AnswerBody::Text {
            lines: vec![docket_core::Reveal::Plain(text.to_owned())],
        },
        progress: Vec::new(),
        footer: footer(),
    }
}

pub fn hit(key: &str) -> Hit {
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

pub fn entry(id: u64, said: &str, state: UndoState) -> UndoEntry {
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

pub fn run_path(id: &str) -> OwnedObjectPath {
    OwnedObjectPath::try_from(format!("/org/quire/Cua1/run/{id}")).unwrap_or_else(|e| panic!("{e}"))
}

pub fn timeline_entry(seq: u64) -> TimelineEntry {
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

/// The stages of a turn as `Routing` carries them: `Answered by Claude Haiku 4.5 via OpenRouter
/// (already loaded)`.
pub fn haiku_notes() -> Vec<companion_wire::RouteNote> {
    route_notes(
        r#"[{"stage":"answer","served":{"account":"openrouter","model":"claude-haiku-4-5","locality":{"kind":"cloud","v":{"region":null}}},"why":[{"kind":"warm"}],"reached":{"provider":"openrouter","door":"gateway"}}]"#,
    )
}

/// Notes from the JSON `Routing` carries.
pub fn route_notes(text: &str) -> Vec<companion_wire::RouteNote> {
    serde_json::from_str(text).unwrap_or_else(|e| panic!("{e}"))
}
