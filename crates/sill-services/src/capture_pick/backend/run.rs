//! The loop: passive until `Serve`, then the calls from transom and the person's answers, each a
//! [`CaptureEvent`] fed to the pure step; what the step asks (a reply) is carried out here.

use std::collections::HashMap;
use std::sync::Arc;

use sill_model::apps::AppIndex;
use sill_model::capture_pick::{
    Admit, AskId, AskedBy, Asking, CaptureCommand, CaptureEvent, CaptureLink, CaptureOut,
    CapturePicker, ColorAsking, ColorReply, OutputCandidate, PermissionAsking, PermissionOutcome,
    Reply, admits, consent_ask, holds_consent, permission_admits, step, windows_of,
};
use sill_model::desktop::{Answer, Consent, ConsentId, DesktopCommand, DesktopView};
use sill_model::displays::Displays;
use tokio::sync::{mpsc, oneshot, watch};
use zbus::Connection;
use zbus::fdo;

use super::objects::{Call, ColorCall, PATH, PORTAL_BUS, PermissionCall, PickerObject, SourceCall};
use crate::callers::CallerSource;
use crate::capture_pick::{CaptureLinks, CaptureService};
use crate::owner;
use crate::seat::SeatView;
use crate::service::ServiceIo;

pub(crate) async fn run(
    links: CaptureLinks,
    io: ServiceIo<CaptureService>,
    callers: Arc<dyn CallerSource>,
) {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    let CaptureLinks {
        seat,
        displays,
        desktop,
        apps,
        answers,
    } = links;
    let (calls_tx, mut calls) = mpsc::unbounded_channel();
    let (left_tx, mut left) = mpsc::unbounded_channel();
    let mut seat = Some(seat);
    let mut displays = Some(displays);
    let mut desktop = Some(desktop);
    let mut live = Live {
        state: CapturePicker::default(),
        tx,
        calls: calls_tx,
        left: left_tx,
        callers,
        apps,
        answers,
        exported: None,
        waiters: HashMap::new(),
        color_waiters: HashMap::new(),
        permission_waiters: HashMap::new(),
        next: 0,
    };
    if let Some(displays) = &displays {
        live.screens(&displays.borrow());
    }
    if let Some(desktop) = &desktop {
        live.windows(&desktop.borrow());
    }
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command, seat.as_ref()),
                None => break,
            },
            Some(call) = calls.recv() => live.call(call),
            Some(()) = left.recv() => live.caller_gone(),
            changed = async {
                match seat.as_mut() {
                    Some(seat) => seat.changed().await,
                    None => std::future::pending().await,
                }
            } => {
                if changed.is_err() {
                    seat = None;
                } else {
                    live.export(seat.as_ref());
                }
            }
            changed = async {
                match displays.as_mut() {
                    Some(displays) => displays.changed().await,
                    None => std::future::pending().await,
                }
            } => {
                if changed.is_err() {
                    displays = None;
                } else if let Some(displays) = &mut displays {
                    live.screens(&displays.borrow_and_update());
                }
            }
            changed = async {
                match desktop.as_mut() {
                    Some(desktop) => desktop.changed().await,
                    None => std::future::pending().await,
                }
            } => {
                if changed.is_err() {
                    desktop = None;
                } else if let Some(desktop) = &mut desktop {
                    live.windows(&desktop.borrow_and_update());
                }
            }
        }
    }
}

struct Live {
    state: CapturePicker,
    tx: watch::Sender<CapturePicker>,
    calls: mpsc::UnboundedSender<Call>,
    left: mpsc::UnboundedSender<()>,
    /// Tells the callers on the exported connection apart.
    callers: Arc<dyn CallerSource>,
    /// The installed apps, which name a program the compositor asks about.
    apps: watch::Receiver<AppIndex>,
    /// Where the compositor hears the person's answer.
    answers: mpsc::UnboundedSender<DesktopCommand>,
    /// The connection the object is exported on.
    exported: Option<Connection>,
    /// Who waits for each open request's answer.
    waiters: HashMap<AskId, oneshot::Sender<fdo::Result<Reply>>>,
    /// Who waits for each open colour request's answer.
    color_waiters: HashMap<AskId, oneshot::Sender<fdo::Result<ColorReply>>>,
    /// Who waits for each open permission question's answer.
    permission_waiters: HashMap<AskId, oneshot::Sender<fdo::Result<PermissionOutcome>>>,
    next: u64,
}

impl Live {
    fn feed(&mut self, event: CaptureEvent) {
        let (next, out) = step(std::mem::take(&mut self.state), event);
        self.tx.send_if_modified(|published| {
            let changed = *published != next;
            if changed {
                published.clone_from(&next);
            }
            changed
        });
        self.state = next;
        for effect in out {
            match effect {
                CaptureOut::Reply(id, reply) => {
                    if let Some(waiter) = self.waiters.remove(&id) {
                        // The caller may have given up; nobody is left to hear it.
                        let _ = waiter.send(Ok(reply));
                    }
                }
                CaptureOut::ColorReply(id, reply) => {
                    if let Some(waiter) = self.color_waiters.remove(&id) {
                        let _ = waiter.send(Ok(reply));
                    }
                }
                CaptureOut::PermissionReply(id, outcome) => {
                    if let Some(waiter) = self.permission_waiters.remove(&id) {
                        let _ = waiter.send(Ok(outcome));
                    }
                }
                CaptureOut::ConsentReply(ask, answer) => self.tell_compositor(ask, answer),
            }
        }
    }

    fn command(&mut self, command: CaptureCommand, seat: Option<&SeatView>) {
        match command {
            CaptureCommand::Serve => {
                self.feed(CaptureEvent::Link(CaptureLink::Serving));

                let connection = seat.and_then(SeatView::current);
                self.export_on(connection);
            }
            CaptureCommand::Outputs(outputs) => {
                let mut candidates = self.state.candidates.clone();
                candidates.outputs = outputs;
                self.feed(CaptureEvent::Candidates(candidates));
            }
            CaptureCommand::Answer(id, outcome) => self.feed(CaptureEvent::Answered(id, outcome)),
            CaptureCommand::Eyedropper(eyedropper) => {
                self.feed(CaptureEvent::Eyedropper(eyedropper));
            }
            CaptureCommand::ColorAnswer(id, outcome) => {
                self.feed(CaptureEvent::ColorAnswered(id, outcome));
            }
            CaptureCommand::PermissionAnswer(id, outcome) => {
                self.feed(CaptureEvent::PermissionAnswered(id, outcome));
            }
            CaptureCommand::Consent(consent) => self.consent(&consent),
            CaptureCommand::ConsentWithdrawn(ask) => {
                self.feed(CaptureEvent::PermissionWithdrawn(ask));
            }
        }
    }

    /// The compositor's question: on the sheet, or waiting its turn. When the line is full the
    /// compositor is told at once that nothing was answered, which it never remembers.
    fn consent(&mut self, consent: &Consent) {
        let full = permission_admits(&self.state) == Admit::Busy;
        if full && !holds_consent(&self.state, consent.ask) {
            self.tell_compositor(consent.ask, Answer::Dismissed);
            return;
        }
        let ask = consent_ask(consent, &self.apps.borrow());
        self.next += 1;
        let id = AskId(self.next);
        let by = AskedBy::Compositor(consent.ask);
        self.feed(CaptureEvent::PermissionOpened(PermissionAsking {
            id,
            ask,
            by,
        }));
    }

    /// Say what the person answered to the compositor's question `ask`. A compositor that has
    /// gone has nobody to hear it.
    fn tell_compositor(&self, ask: ConsentId, answer: Answer) {
        let _ = self
            .answers
            .send(DesktopCommand::AnswerPermission { ask, answer });
    }

    /// The displays as screens.
    fn screens(&mut self, displays: &Displays) {
        let outputs = displays
            .outputs
            .iter()
            .map(|output| OutputCandidate {
                name: output.name.0.clone(),
                title: if output.product.is_empty() {
                    output.name.0.clone()
                } else {
                    output.product.clone()
                },
            })
            .collect();
        self.command(CaptureCommand::Outputs(outputs), None);
    }

    /// The desktop's windows as the ones the picker offers, by the identifiers casement gave.
    fn windows(&mut self, desktop: &DesktopView) {
        let mut candidates = self.state.candidates.clone();
        candidates.windows = windows_of(desktop);
        self.feed(CaptureEvent::Candidates(candidates));
    }

    /// The seat changed: serve on its new connection when serving.
    fn export(&mut self, seat: Option<&SeatView>) {
        if self.state.link == CaptureLink::Serving {
            self.export_on(seat.and_then(SeatView::current));
        }
    }

    fn export_on(&mut self, connection: Option<Connection>) {
        let Some(connection) = connection else {
            return;
        };
        if self
            .exported
            .as_ref()
            .is_some_and(|held| held.unique_name() == connection.unique_name())
        {
            return;
        }
        self.exported = Some(connection.clone());
        let calls = self.calls.clone();
        let left = self.left.clone();
        let peers = self.callers.peers_on(connection.clone());
        tokio::spawn(async move {
            let exported = connection
                .object_server()
                .at(PATH, PickerObject { calls, peers })
                .await;
            if let Err(error) = exported {
                eprintln!("sill: capture picker: serving {PATH} failed: {error}");
                return;
            }
            watch_caller(connection, left).await;
        });
    }

    fn call(&mut self, call: Call) {
        match call {
            Call::Sources(call) => self.sources_call(call),
            Call::Color(call) => self.color_call(call),
            Call::Permission(call) => self.permission_call(call),
        }
    }

    fn permission_call(&mut self, call: PermissionCall) {
        let PermissionCall { ask, answer } = call;
        match permission_admits(&self.state) {
            Admit::Busy => {
                let _ = answer.send(Err(fdo::Error::LimitsExceeded(
                    "too many questions are waiting".to_owned(),
                )));
            }
            Admit::Yes => {
                self.next += 1;
                let id = AskId(self.next);
                self.permission_waiters.insert(id, answer);
                let by = AskedBy::Portal;
                self.feed(CaptureEvent::PermissionOpened(PermissionAsking {
                    id,
                    ask,
                    by,
                }));
            }
        }
    }

    fn color_call(&mut self, call: ColorCall) {
        let ColorCall { ask, answer } = call;
        match admits(&self.state) {
            Admit::Busy => {
                let _ = answer.send(Err(fdo::Error::LimitsExceeded(
                    "a question is already open".to_owned(),
                )));
            }
            Admit::Yes => {
                self.next += 1;
                let id = AskId(self.next);
                self.color_waiters.insert(id, answer);
                self.feed(CaptureEvent::ColorOpened(ColorAsking { id, ask }));
            }
        }
    }

    fn sources_call(&mut self, call: SourceCall) {
        let SourceCall { ask, answer } = call;
        match admits(&self.state) {
            Admit::Busy => {
                let _ = answer.send(Err(fdo::Error::LimitsExceeded(
                    "a sharing question is already open".to_owned(),
                )));
            }
            Admit::Yes => {
                self.next += 1;
                let id = AskId(self.next);
                self.waiters.insert(id, answer);
                self.feed(CaptureEvent::Opened(Asking { id, ask }));
            }
        }
    }

    /// The caller left the bus or was replaced: nobody waits any more.
    fn caller_gone(&mut self) {
        self.waiters.clear();
        self.color_waiters.clear();
        self.permission_waiters.clear();
        self.feed(CaptureEvent::CallerGone);
    }
}

/// Tell the loop each time the portal backend leaves or is replaced.
async fn watch_caller(connection: Connection, left: mpsc::UnboundedSender<()>) {
    owner::while_owned(
        &connection,
        PORTAL_BUS,
        || left.closed(),
        || {
            // A closed receiver ends the next task's wait, so a lost send is no loss.
            let _ = left.send(());
        },
    )
    .await;
}
