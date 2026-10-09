//! The service's loop. Commands and what the bus says go through the pure `step`, whose state is
//! published and whose effects are carried out here: `Begin` on its own task, `Release` and
//! `Cancel` on the utterance object, the question to the companion.
//!
//! The utterance is followed by its stream (frames, one reader task) and by its `Ended` signal
//! (one subscription for the seat's whole life, made before the first `Begin`). An `Ended` that
//! arrives while `Begin` is still on the wire is held until the object's path is known.

use std::os::fd::OwnedFd;
use std::sync::Arc;

use docket_core::VoiceIntent;
use prov::{SpaceId, UnixSeconds};
use sill_model::companion::CompanionCommand;
use sill_model::voice::{
    ConsentAnswer, Voice, VoiceConsent, VoiceEffect, VoiceEvent, VoiceLink, step,
};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
use voice_dbus::{UtteranceProxy, VoiceProxy};
use voice_wire::{
    CancelCause, UtteranceEnd, VoiceBegin, VoiceEvent as Frame, VoiceRefusal, VoiceTrigger, seal,
};
use zbus::Connection;
use zbus::zvariant::OwnedObjectPath;

use super::{reader, signals};
use crate::control::Control;
use crate::env::{Env, WallClock};
use crate::seat::SeatView;
use crate::service::ServiceIo;
use crate::voice::{VoiceLinks, VoiceService};

/// The error voiced answers a `Cancel` whose cause it will not take (superseded, too long, empty,
/// bad JSON): the microphone stays open.
const MALFORMED: &str = "org.quire.Voice1.Error.Malformed";

/// Which utterance a frame belongs to: a stale reader's frames are not the new hold's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Serial(u64);

/// What the loop's tasks tell it.
#[derive(Debug)]
pub(super) enum Inbound {
    /// A step's event.
    Event(VoiceEvent),
    /// `Begin` returned.
    Opened(Serial, OwnedObjectPath, UnixStream),
    /// A frame of utterance `Serial`'s stream.
    Frame(Serial, Frame),
    /// Utterance `Serial`'s stream closed.
    Closed(Serial),
    /// Utterance `Serial`'s stream sent a frame over the cap and was closed.
    TooLong(Serial),
    /// `Ended` of the object at this path.
    Signal(String, UtteranceEnd),
}

/// The utterance being followed.
enum Utterance {
    None,
    /// `Begin` is on the wire; `Ended` signals that came ahead of its reply wait here.
    Beginning(Serial, Vec<(String, UtteranceEnd)>),
    Open {
        serial: Serial,
        path: OwnedObjectPath,
        reader: JoinHandle<()>,
    },
}

struct Live {
    state: Voice,
    tx: watch::Sender<Voice>,
    connection: Option<Connection>,
    utterance: Utterance,
    next_serial: u64,
    companion: Control<CompanionCommand>,
    consents: mpsc::UnboundedSender<VoiceConsent>,
    clock: Arc<dyn WallClock>,
    inbound: mpsc::UnboundedSender<Inbound>,
    signals: Option<JoinHandle<()>>,
}

/// What the loop reaches outside the process: the wall clock the consent is stamped with.
pub(crate) struct Reach {
    clock: Arc<dyn WallClock>,
}

impl Reach {
    pub(crate) fn of(env: &Env) -> Reach {
        Reach {
            clock: Arc::clone(&env.clock),
        }
    }
}

pub(crate) async fn run(reach: Reach, links: VoiceLinks, io: ServiceIo<VoiceService>) {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    let (inbound_tx, mut inbound) = mpsc::unbounded_channel();
    let VoiceLinks {
        mut seat,
        companion,
        consents,
    } = links;
    let mut live = Live {
        state: Voice::default(),
        tx,
        connection: None,
        utterance: Utterance::None,
        next_serial: 0,
        companion,
        consents,
        clock: reach.clock,
        inbound: inbound_tx,
        signals: None,
    };
    live.seated(seat.current()).await;
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.feed(VoiceEvent::Told(command)),
                None => break,
            },
            Some(message) = inbound.recv() => live.inbound(message),
            changed = seat.changed() => {
                if changed.is_err() {
                    break;
                }
                live.seated(SeatView::current(&seat)).await;
            }
        }
    }
    live.seated(None).await;
}

impl Live {
    /// The seat was taken or given up: subscribe to `Ended` first, then say the link is ready.
    async fn seated(&mut self, connection: Option<Connection>) {
        if let Some(task) = self.signals.take() {
            task.abort();
        }
        self.connection = connection.clone();
        let Some(connection) = connection else {
            return self.feed(VoiceEvent::Link(VoiceLink::Passive));
        };
        match signals::subscribe(&connection).await {
            Ok(stream) => {
                self.signals = Some(tokio::spawn(signals::forward(
                    connection,
                    stream,
                    self.inbound.clone(),
                )));
                self.feed(VoiceEvent::Link(VoiceLink::Ready));
            }
            Err(error) => {
                eprintln!("sill: voice: cannot watch for the end of an utterance: {error}");
                self.feed(VoiceEvent::Link(VoiceLink::Passive));
            }
        }
    }

    fn inbound(&mut self, message: Inbound) {
        match message {
            Inbound::Event(event) => self.feed(event),
            Inbound::Opened(serial, path, stream) => self.opened(serial, path, stream),
            Inbound::Frame(serial, frame) if self.current(serial) => {
                self.feed(VoiceEvent::Frame(frame));
            }
            Inbound::Closed(serial) if self.current(serial) => self.feed(VoiceEvent::StreamClosed),
            Inbound::TooLong(serial) if self.current(serial) => {
                self.feed(VoiceEvent::StreamBroken);
            }
            Inbound::Frame(..) | Inbound::Closed(_) | Inbound::TooLong(_) => {}
            Inbound::Signal(path, end) => self.signalled(path, end),
        }
    }

    fn current(&self, serial: Serial) -> bool {
        match &self.utterance {
            Utterance::Open { serial: open, .. } => *open == serial,
            Utterance::None | Utterance::Beginning(..) => false,
        }
    }

    /// `Begin` returned: follow the stream, tell the machine, then replay what `Ended` came early.
    fn opened(&mut self, serial: Serial, path: OwnedObjectPath, stream: UnixStream) {
        let Utterance::Beginning(began, early) =
            std::mem::replace(&mut self.utterance, Utterance::None)
        else {
            return;
        };
        if began != serial {
            return;
        }
        let reader = tokio::spawn(reader::read(stream, serial, self.inbound.clone()));
        self.utterance = Utterance::Open {
            serial,
            path: path.clone(),
            reader,
        };
        self.feed(VoiceEvent::Began);
        for (at, end) in early {
            if at == path.as_str() {
                self.signalled(at, end);
            }
        }
    }

    fn signalled(&mut self, path: String, end: UtteranceEnd) {
        match &mut self.utterance {
            Utterance::Beginning(_, early) => early.push((path, end)),
            Utterance::Open { path: open, .. } if open.as_str() == path => {
                self.feed(VoiceEvent::Ended(end));
            }
            Utterance::Open { .. } | Utterance::None => {}
        }
    }

    /// Step the state by `event`, publish it when it changed and carry out what it asks.
    fn feed(&mut self, event: VoiceEvent) {
        let (next, effects) = step(self.state.clone(), event);
        if next != self.state {
            self.tx.send_replace(next.clone());
        }
        let idle = matches!(next.phase, sill_model::voice::Phase::Idle);
        self.state = next;
        for effect in effects {
            self.carry_out(effect);
        }
        if idle {
            self.forget();
        }
    }

    /// Nothing is held: stop following the utterance.
    fn forget(&mut self) {
        if let Utterance::Open { reader, .. } = &self.utterance {
            reader.abort();
        }
        self.utterance = Utterance::None;
    }

    fn carry_out(&mut self, effect: VoiceEffect) {
        match effect {
            VoiceEffect::Begin(trigger) => self.begin(trigger),
            VoiceEffect::Release => self.call(Call::Release),
            VoiceEffect::Cancel(cause) => self.call(Call::Cancel(cause)),
            VoiceEffect::Record(answer) => {
                let consent = match answer {
                    ConsentAnswer::Allow => {
                        VoiceConsent::Given(UnixSeconds(self.clock.now().timestamp().as_second()))
                    }
                    ConsentAnswer::NotNow => VoiceConsent::Declined,
                };
                let _ = self.consents.send(consent);
            }
            VoiceEffect::Ask(input) => {
                let _ = self.companion.send(CompanionCommand::Ask(input));
            }
        }
    }

    fn begin(&mut self, trigger: VoiceTrigger) {
        self.next_serial += 1;
        let serial = Serial(self.next_serial);
        self.utterance = Utterance::Beginning(serial, Vec::new());
        let inbound = self.inbound.clone();
        let Some(connection) = self.connection.clone() else {
            let _ = inbound.send(Inbound::Event(VoiceEvent::Unreachable));
            return;
        };
        tokio::spawn(async move {
            let message = match ask_begin(&connection, trigger).await {
                Ok((path, stream)) => Inbound::Opened(serial, path, stream),
                Err(event) => Inbound::Event(event),
            };
            let _ = inbound.send(message);
        });
    }

    fn call(&mut self, call: Call) {
        let (Some(connection), Utterance::Open { path, .. }) =
            (self.connection.clone(), &self.utterance)
        else {
            return;
        };
        let path = path.clone();
        let inbound = self.inbound.clone();
        tokio::spawn(async move {
            // Voiced refusing a cancel as malformed leaves the microphone open: say so. Any
            // other failure means voiced is gone, and nothing is left to cancel.
            if let Err(zbus::Error::MethodError(name, ..)) = call.on(&connection, path).await
                && name.as_str() == MALFORMED
            {
                let _ = inbound.send(Inbound::Event(VoiceEvent::CancelRefused));
            }
        });
    }
}

/// What is said to the utterance object.
enum Call {
    Release,
    Cancel(CancelCause),
}

impl Call {
    async fn on(self, connection: &Connection, path: OwnedObjectPath) -> zbus::Result<()> {
        let utterance = UtteranceProxy::builder(connection)
            .path(path)?
            .build()
            .await?;
        match self {
            Call::Release => utterance.release().await,
            Call::Cancel(cause) => {
                let sealed = seal(&cause).map_err(|e| zbus::Error::Failure(e.to_string()))?;
                utterance.cancel(&sealed).await
            }
        }
    }
}

/// `Voice1.Begin` for an ask over the desktop's Space: the utterance object and its stream, or
/// the event that says why not.
async fn ask_begin(
    connection: &Connection,
    trigger: VoiceTrigger,
) -> Result<(OwnedObjectPath, UnixStream), VoiceEvent> {
    let begin = VoiceBegin {
        intent: VoiceIntent::Ask,
        trigger,
        space: SpaceId::desktop(),
    };
    let body = seal(&begin).map_err(|_| VoiceEvent::Unreachable)?;
    let voice = VoiceProxy::new(connection)
        .await
        .map_err(|_| VoiceEvent::Unreachable)?;
    match voice.begin(&body).await {
        Ok((path, fd)) => Ok((path, stream_of(fd.into())?)),
        Err(zbus::Error::MethodError(name, ..)) => {
            Err(VoiceRefusal::from_error_name(name.as_str())
                .map_or(VoiceEvent::Unreachable, VoiceEvent::Refused))
        }
        Err(_) => Err(VoiceEvent::Unreachable),
    }
}

/// The stream's socket, for the reactor.
fn stream_of(fd: OwnedFd) -> Result<UnixStream, VoiceEvent> {
    let socket = std::os::unix::net::UnixStream::from(fd);
    socket
        .set_nonblocking(true)
        .and_then(|()| UnixStream::from_std(socket))
        .map_err(|_| VoiceEvent::Unreachable)
}
