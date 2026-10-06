//! The voice service's test bench: a private bus, a stand-in `voiced` that owns
//! `org.quire.Voice1` and writes framed events to the stream it hands back, and the desk that
//! holds the service under test and the connection the shell's seat would publish. Everything
//! waits on the service's published state or on the stand-in's record, never on a clock.
#![allow(
    dead_code,
    unused_macros,
    reason = "each test binary uses part of the bench"
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sill_model::companion::CompanionCommand;
use sill_model::voice::{Voice, VoiceCommand, VoiceConsent, VoiceLink};
use sill_services::env::{BusRoute, Buses, Env};
use sill_services::seat::{SeatPublisher, seat};
use sill_services::service::{Handle, spawn};
use sill_services::{Control, VoiceLinks, VoiceService};
use sill_testkit::PrivateBus;
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::sync::{mpsc, watch};
use voice_wire::{UtteranceEnd, VoiceEvent as Frame, VoiceRefusal, frame, seal};
use zbus::message::Header;
use zbus::zvariant::{OwnedFd, OwnedObjectPath};
use zbus::{Connection, DBusError};

/// A failure bound far above any real delivery, not a delay anything depends on.
const GIVE_UP: Duration = Duration::from_secs(60);

/// What `Begin` does.
#[derive(Debug, Clone)]
pub enum BeginAnswer {
    /// Open an utterance.
    Open,
    /// Refuse.
    Refuse(VoiceRefusal),
    /// Decide as `voiced` does: from the `[voice]` table of this settings file.
    FromSettings(PathBuf),
}

/// The stand-in's script and record.
pub struct Script {
    pub answer: Mutex<BeginAnswer>,
    /// `Ended` to emit, unicast, from inside `Begin` before it replies.
    pub early_end: Mutex<Option<UtteranceEnd>>,
    streams: Mutex<Vec<UnixStream>>,
    log: watch::Sender<Vec<String>>,
}

impl Script {
    fn new(answer: BeginAnswer) -> Arc<Script> {
        Arc::new(Script {
            answer: Mutex::new(answer),
            early_end: Mutex::new(None),
            streams: Mutex::new(Vec::new()),
            log: watch::Sender::new(Vec::new()),
        })
    }

    fn push(&self, line: impl Into<String>) {
        self.log.send_modify(|lines| lines.push(line.into()));
    }

    pub fn lines(&self) -> Vec<String> {
        self.log.borrow().clone()
    }

    /// Resolves with what `ready` makes of the record, once it makes something.
    pub async fn until<T>(&self, ready: impl Fn(&[String]) -> Option<T>) -> T {
        let mut lines = self.log.subscribe();
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
            _ => panic!(
                "the stand-in never recorded it; the record is {:?}",
                self.lines()
            ),
        }
    }

    /// One frame down the newest utterance's stream.
    pub async fn write(&self, event: &Frame) {
        let bytes = frame(event).unwrap_or_else(|e| panic!("{e}"));
        let mut stream = self
            .streams
            .lock()
            .unwrap_or_else(|e| panic!("{e}"))
            .pop()
            .unwrap_or_else(|| panic!("no utterance is open"));
        stream
            .write_all(&bytes)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
        self.streams
            .lock()
            .unwrap_or_else(|e| panic!("{e}"))
            .push(stream);
    }
}

/// The refusals as bus errors of `org.quire.Voice1.Error.*`.
#[derive(Debug, DBusError)]
#[zbus(prefix = "org.quire.Voice1.Error")]
enum Refused {
    #[zbus(error)]
    ZBus(zbus::Error),
    Disabled(String),
    NeedsConsent(String),
    Busy(String),
    NotAllowed(String),
    NoModel(String),
    MicUnavailable(String),
}

impl From<VoiceRefusal> for Refused {
    fn from(refusal: VoiceRefusal) -> Refused {
        let why = refusal.to_string();
        match refusal {
            VoiceRefusal::Disabled => Refused::Disabled(why),
            VoiceRefusal::NeedsConsent => Refused::NeedsConsent(why),
            VoiceRefusal::Busy => Refused::Busy(why),
            VoiceRefusal::NotAllowed => Refused::NotAllowed(why),
            VoiceRefusal::NoModel => Refused::NoModel(why),
            VoiceRefusal::MicUnavailable => Refused::MicUnavailable(why),
        }
    }
}

/// What `voiced` makes of a settings file: `hold_to_talk = "off"` is off, `consent` of `given`
/// (or a table with a `given` key) is on, `declined` is off, anything else needs consent.
fn refusal_of_settings(path: &Path) -> Option<VoiceRefusal> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let table: toml::Table = text.parse().unwrap_or_default();
    let Some(voice) = table.get("voice").and_then(toml::Value::as_table) else {
        return Some(VoiceRefusal::NeedsConsent);
    };
    if voice.get("hold_to_talk").and_then(toml::Value::as_str) == Some("off") {
        return Some(VoiceRefusal::Disabled);
    }
    match voice.get("consent") {
        Some(toml::Value::String(s)) if s == "given" => None,
        Some(toml::Value::Table(t)) if t.contains_key("given") => None,
        Some(toml::Value::String(s)) if s == "declined" => Some(VoiceRefusal::Disabled),
        _ => Some(VoiceRefusal::NeedsConsent),
    }
}

struct StandInVoice {
    script: Arc<Script>,
    next: Mutex<u64>,
}

#[zbus::interface(name = "org.quire.Voice1")]
impl StandInVoice {
    async fn begin(
        &self,
        begin: String,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> Result<(OwnedObjectPath, OwnedFd), Refused> {
        self.script.push(format!("begin {begin}"));
        let refusal = match self
            .script
            .answer
            .lock()
            .unwrap_or_else(|e| panic!("{e}"))
            .clone()
        {
            BeginAnswer::Open => None,
            BeginAnswer::Refuse(refusal) => Some(refusal),
            BeginAnswer::FromSettings(path) => refusal_of_settings(&path),
        };
        if let Some(refusal) = refusal {
            return Err(refusal.into());
        }
        let n = {
            let mut next = self.next.lock().unwrap_or_else(|e| panic!("{e}"));
            *next += 1;
            *next
        };
        let path = format!("/org/quire/Voice1/utterance/{n}");
        let (near, far) = UnixStream::pair().unwrap_or_else(|e| panic!("{e}"));
        self.script
            .streams
            .lock()
            .unwrap_or_else(|e| panic!("{e}"))
            .push(near);
        let early = self
            .script
            .early_end
            .lock()
            .unwrap_or_else(|e| panic!("{e}"))
            .take();
        if let Some(end) = early {
            connection
                .emit_signal(
                    header.sender().cloned(),
                    path.as_str(),
                    "org.quire.Voice1.Utterance",
                    "Ended",
                    &(seal(&end).unwrap_or_else(|e| panic!("{e}")),),
                )
                .await?;
        }
        let far = far.into_std().unwrap_or_else(|e| panic!("{e}"));
        let fd: std::os::fd::OwnedFd = far.into();
        Ok((
            OwnedObjectPath::try_from(path).unwrap_or_else(|e| panic!("{e}")),
            fd.into(),
        ))
    }
}

struct StandInUtterance {
    script: Arc<Script>,
}

#[zbus::interface(name = "org.quire.Voice1.Utterance")]
impl StandInUtterance {
    async fn release(&self) {
        self.script.push("release");
    }

    async fn cancel(&self) {
        self.script.push("cancel");
    }
}

/// The service under test, its bus, and the stand-in voiced.
pub struct Desk {
    pub bus: PrivateBus,
    pub voice: Handle<VoiceService>,
    pub seat: SeatPublisher,
    pub script: Arc<Script>,
    /// What the service asked the companion.
    pub asks: mpsc::UnboundedReceiver<CompanionCommand>,
    /// The consent the service asked to have written.
    pub consents: mpsc::UnboundedReceiver<VoiceConsent>,
    pub shell: Connection,
    _voiced: Connection,
}

pub async fn connect(address: &str) -> Connection {
    zbus::connection::Builder::address(address)
        .unwrap_or_else(|e| panic!("{e}"))
        .build()
        .await
        .unwrap_or_else(|e| panic!("connecting to the private bus: {e}"))
}

/// A desk whose stand-in voiced answers `Begin` as `answer` says; `None` (announced) without a
/// `dbus-daemon`.
pub async fn desk(answer: BeginAnswer) -> Option<Desk> {
    let bus = PrivateBus::start()?;
    let env = Env {
        buses: Buses::new(BusRoute::Absent, BusRoute::Address(bus.address.clone())),
        ..Env::isolated(Path::new("/nonexistent/sill-voice-bus"))
    };
    let script = Script::new(answer);
    let voiced = connect(&bus.address).await;
    voiced
        .object_server()
        .at(
            "/org/quire/Voice1",
            StandInVoice {
                script: Arc::clone(&script),
                next: Mutex::new(0),
            },
        )
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    for n in 1..=4 {
        voiced
            .object_server()
            .at(
                format!("/org/quire/Voice1/utterance/{n}"),
                StandInUtterance {
                    script: Arc::clone(&script),
                },
            )
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }
    voiced
        .request_name("org.quire.Voice1")
        .await
        .unwrap_or_else(|e| panic!("{e}"));
    let (seat_publisher, seat_view) = seat();
    let (companion, asks) = Control::<CompanionCommand>::channel();
    let (consents_tx, consents) = mpsc::unbounded_channel();
    let voice = spawn::<VoiceService>(
        &tokio::runtime::Handle::current(),
        &env,
        VoiceLinks {
            seat: seat_view,
            companion,
            consents: consents_tx,
        },
    );
    let shell = connect(&bus.address).await;
    Some(Desk {
        bus,
        voice,
        seat: seat_publisher,
        script,
        asks,
        consents,
        shell,
        _voiced: voiced,
    })
}

impl Desk {
    /// The shell takes its seat; the service subscribes and says it is ready.
    pub async fn seated(&mut self) {
        self.seat.publish(self.shell.clone());
        self.state(|v| matches!(v.link, VoiceLink::Ready).then_some(()))
            .await;
    }

    pub fn send(&self, command: VoiceCommand) {
        self.voice
            .control
            .send(command)
            .unwrap_or_else(|e| panic!("{e}"));
    }

    /// The service's state, once `ready` says yes to it.
    pub async fn state<T>(&self, ready: impl Fn(&Voice) -> Option<T>) -> T {
        let mut rx = self.voice.state.clone();
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
            _ => panic!(
                "the service never reached it; it is at {:?}",
                self.voice.state.borrow()
            ),
        }
    }

    /// The next question the service asked the companion.
    pub async fn next_ask(&mut self) -> CompanionCommand {
        tokio::time::timeout(GIVE_UP, self.asks.recv())
            .await
            .unwrap_or_else(|_| panic!("no ask came"))
            .unwrap_or_else(|| panic!("the companion link closed"))
    }
}

macro_rules! desk {
    ($answer:expr) => {
        match $crate::voice_support::desk($answer).await {
            Some(desk) => desk,
            None => return,
        }
    };
}
