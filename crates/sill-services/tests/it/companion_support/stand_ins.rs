//! The stand-ins the companion service talks to on a private bus: intentd's session, search,
//! control and run objects, companiond and its answer, an app that serves the summon, cuad's
//! manager and runs, and memoryd. Each records what it was asked on the shared [`Log`].

use std::sync::{Arc, Mutex};

use almanac_core::{FactView, TimelineEntry, TimelinePage};
use docket_core::{Displayed, Hit, SessionOpened, SummonAnswer, UndoEntry, UndoFault};
use docket_dbus::answer_path;
use prov::{DataClass, Label, SessionId, Source, SpaceId};
use zbus::fdo;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{ObjectPath, OwnedObjectPath};
use zbus::{Connection, interface};

use super::{Details, Log, json, task};
use docket_core::{
    CallRefusal, Delivery, Follow, Outcome, Preview, SendRefusal, Undoable, ValidManifest,
};
use prov::{Crossing, MessageId, ThreadId};

// ---- intentd ----

pub struct StandInSession(pub Log);

#[interface(name = "org.quire.Intents1.Session")]
impl StandInSession {
    async fn turn(&self, session: String, turn: String, _options: Details) -> fdo::Result<u64> {
        self.0.push(format!("turn {session} {turn}"));
        Ok(41)
    }

    /// The labelled reading the shell asks for first. Handle [`MAIL_HANDLE`] holds somebody
    /// else's words (mail); every other handle is the person's own.
    async fn display_labelled(&self, session: String, handle: u64) -> fdo::Result<String> {
        self.0.push(format!("display {session} {handle}"));
        if self.0.lines().contains(&format!("close {session}")) {
            return Err(fdo::Error::Failed("NoSuchSession".to_owned()));
        }
        let label = match handle {
            MAIL_HANDLE => Label::untrusted(Source::Mail, DataClass::Mail, SpaceId::desktop()),
            _ => Label::trusted_user(),
        };
        let shown = Displayed {
            text: format!("the text of handle {handle}"),
            label,
        };
        Ok(json(&shown))
    }
}

/// The handle whose words are somebody else's.
pub const MAIL_HANDLE: u64 = 9;

/// A router from before the labelled reading: it has `Display` alone, so a `DisplayLabelled`
/// call is an unknown method.
pub struct StandInOlderSession(pub Log);

#[interface(name = "org.quire.Intents1.Session")]
impl StandInOlderSession {
    async fn turn(&self, session: String, turn: String, _options: Details) -> fdo::Result<u64> {
        self.0.push(format!("turn {session} {turn}"));
        Ok(41)
    }

    async fn display(&self, session: String, handle: u64) -> fdo::Result<String> {
        self.0.push(format!("display {session} {handle}"));
        Ok(format!("the text of handle {handle}"))
    }
}

/// What the stand-in message service answers a send: delivered, in the sender's Space.
pub struct StandInMessage(pub Log);

#[interface(name = "org.quire.Intents1.Message")]
impl StandInMessage {
    async fn send(&self, session: String, draft: String, _options: Details) -> fdo::Result<String> {
        self.0.push(format!("send {session} {draft}"));
        let delivery: Result<Delivery, SendRefusal> = Ok(Delivery {
            message: MessageId::parse("m-1").unwrap_or_else(|e| panic!("{e}")),
            thread: ThreadId::parse("m-1").unwrap_or_else(|e| panic!("{e}")),
            crossing: Crossing::Within,
        });
        Ok(json(&delivery))
    }
}

/// The installed manifests, as the registry lists them.
pub struct StandInRegistry(pub Vec<ValidManifest>);

#[interface(name = "org.quire.Intents1.Registry")]
impl StandInRegistry {
    async fn manifests(&self) -> Vec<(String, String)> {
        self.0
            .iter()
            .map(|valid| (valid.manifest().app.as_str().to_owned(), json(valid)))
            .collect()
    }

    #[zbus(signal)]
    async fn manifest_changed(emitter: &SignalEmitter<'_>, app: &str) -> zbus::Result<()>;
}

#[derive(Clone)]
pub struct StandInSearch {
    pub log: Log,
    pub hits: Arc<Mutex<Vec<Hit>>>,
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

pub struct StandInControl(pub Arc<Mutex<Vec<UndoEntry>>>);

#[interface(name = "org.quire.Intents1.Control")]
impl StandInControl {
    async fn journal(&self, _filter: String) -> fdo::Result<String> {
        Ok(json(&*self.0.lock().unwrap_or_else(|e| panic!("{e}"))))
    }

    #[zbus(signal)]
    async fn journal_changed(emitter: &SignalEmitter<'_>, rows: u64) -> zbus::Result<()>;
}

pub struct StandInRun(pub Log);

#[interface(name = "org.quire.Intents1.Run")]
impl StandInRun {
    async fn perform(
        &self,
        call: String,
        session: String,
        parent_window: String,
        _options: Details,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> fdo::Result<OwnedObjectPath> {
        self.0
            .push(format!("perform {call} {session} {parent_window}"));
        let path = OwnedObjectPath::try_from("/org/quire/Intents1/request/2")
            .map_err(|e| fdo::Error::Failed(e.to_string()))?;
        let to = header.sender().map(ToString::to_string);
        let (connection, at) = (connection.clone(), path.clone());
        tokio::spawn(async move {
            let end: Result<Outcome, CallRefusal> = Ok(Outcome {
                value: None,
                said: None,
                show: Preview::None,
                undo: Undoable::No,
                follow: Follow::Nothing,
            });
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

/// Whether the stand-in companion takes a card press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActsAs {
    /// It answers with the request's path.
    Accepts,
    /// It answers `AccessDenied`.
    Refuses,
}

pub struct StandInCompanion {
    /// What the answers it makes say as their `Session`.
    pub session: Option<String>,
    pub log: Log,
    pub view: Arc<Mutex<String>>,
    pub acts: ActsAs,
    /// What the answers it makes say as their `Routing`.
    pub routing: Option<Arc<Mutex<String>>>,
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
                    session: self.session.clone(),
                    log: self.log.clone(),
                    view: self.view.clone(),
                    acts: self.acts,
                    routing: self.routing.clone(),
                },
            )
            .await?;
        Ok(path)
    }

    async fn told(&self, agent: String, space: String, turn: String) {
        self.log.push(format!("told {agent} {space} {turn}"));
    }

    async fn close(&self, session: String) {
        self.log.push(format!("close {session}"));
    }

    #[zbus(signal)]
    async fn answer_added(emitter: &SignalEmitter<'_>, answer: ObjectPath<'_>) -> zbus::Result<()>;
}

pub struct StandInAnswer {
    /// The `Session` property; `None` is an older companion, whose object has no such property.
    pub session: Option<String>,
    pub log: Log,
    pub view: Arc<Mutex<String>>,
    pub acts: ActsAs,
    /// The `Routing` property's JSON; `None` is an older companion, whose object has no such
    /// property.
    pub routing: Option<Arc<Mutex<String>>>,
}

#[interface(name = "org.quire.Companion1.Answer")]
impl StandInAnswer {
    async fn act(&self, action: String) -> fdo::Result<OwnedObjectPath> {
        self.log.push(format!("act {action}"));
        match self.acts {
            ActsAs::Accepts => OwnedObjectPath::try_from("/org/quire/Companion1/request/1")
                .map_err(|e| fdo::Error::Failed(e.to_string())),
            ActsAs::Refuses => Err(fdo::Error::AccessDenied("not the shell".to_owned())),
        }
    }

    async fn cancel(&self) {
        self.log.push("cancel");
    }

    #[zbus(property)]
    async fn session(&self) -> fdo::Result<String> {
        self.session
            .clone()
            .ok_or_else(|| fdo::Error::UnknownProperty("Session".to_owned()))
    }

    #[zbus(property)]
    async fn routing(&self) -> fdo::Result<String> {
        self.routing
            .as_ref()
            .map(|routing| routing.lock().unwrap_or_else(|e| panic!("{e}")).clone())
            .ok_or_else(|| fdo::Error::UnknownProperty("Routing".to_owned()))
    }

    #[zbus(property)]
    async fn view(&self) -> String {
        self.view.lock().unwrap_or_else(|e| panic!("{e}")).clone()
    }

    #[zbus(signal)]
    async fn updated(emitter: &SignalEmitter<'_>, view: &str) -> zbus::Result<()>;
}

// ---- an app ----

pub struct StandInApp {
    pub answer: Option<SummonAnswer>,
    pub log: Log,
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

#[derive(Default)]
pub struct StandInManager {
    pub runs: Vec<OwnedObjectPath>,
    pub sight: Arc<Mutex<String>>,
}

#[interface(name = "org.quire.Cua1.Manager")]
impl StandInManager {
    async fn runs(&self) -> Vec<OwnedObjectPath> {
        self.runs.clone()
    }

    #[zbus(property)]
    async fn sight_note(&self) -> String {
        self.sight.lock().unwrap_or_else(|e| panic!("{e}")).clone()
    }

    #[zbus(signal)]
    async fn run_removed(emitter: &SignalEmitter<'_>, run: OwnedObjectPath) -> zbus::Result<()>;
}

#[derive(Clone)]
pub struct StandInRunObject {
    pub id: &'static str,
    pub log: Log,
    pub state: Arc<Mutex<String>>,
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
    async fn id(&self) -> String {
        self.id.replace('_', "-")
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

pub struct StandInMemory(pub Vec<TimelineEntry>);

#[interface(name = "org.quire.Memory1.Control")]
impl StandInMemory {
    async fn timeline(&self, _space: String, _query: String) -> String {
        json(&TimelinePage {
            entries: self.0.clone(),
            next: None,
        })
    }
}

/// memoryd with facts waiting for the person: it lists them and logs how each was settled.
pub struct StandInWaiting {
    pub facts: Vec<FactView>,
    pub log: Log,
}

#[interface(name = "org.quire.Memory1.Control")]
impl StandInWaiting {
    async fn timeline(&self, _space: String, _query: String) -> String {
        json(&TimelinePage {
            entries: Vec::new(),
            next: None,
        })
    }

    /// The desktop and a "work" Space; what waits is in "work", where a helper's job remembers.
    async fn spaces(&self) -> String {
        let space = |id: &str| {
            serde_json::json!({
                "id": id, "state": {"kind": "open"}, "vault": "sealed", "created": 0
            })
        };
        serde_json::json!([space("desktop"), space("work")]).to_string()
    }

    async fn pending(&self, space: String) -> String {
        match space.as_str() {
            "work" => json(&self.facts),
            _ => "[]".to_owned(),
        }
    }

    async fn settle(&self, fact: String, verdict: String) {
        self.log.push(format!("settle {fact} {verdict}"));
    }
}
