//! The service's loop.
//!
//! Passive until `Serve`; then [`serve::take`] claims the names and the state says `Serving` (or
//! why not). From there every thing that happens is an [`IntentsEvent`] fed to the pure `step`:
//! a `Confirm` call (given its asker, remembered by serial), a `Cancel`, the person's answer
//! (stamped with the wall clock), a question's expiry (a sleep of its own), intentd arriving or
//! leaving (the owner of `org.quire.Intents1`: arriving reads the grants, leaving drops the
//! questions nobody waits for), and the control's signals. What `step` asks of the outside is
//! carried out here: the `Response` signal to the asker alone, the sleeps, the reads.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use docket_core::{ActionRef, StandingGrantId};
use docket_dbus::INTENTS_BUS;
use prov::UnixSeconds;
use sill_model::intents::sheet::{Reply, Serial, SheetChoice, SheetEffect, SheetEvent};
use sill_model::intents::terminal::{Breaker, TerminalEvent};
use sill_model::intents::{
    Intents, IntentsCommand, IntentsEffect, IntentsEvent, IntentsLink, step,
};
use tokio::sync::{mpsc, watch};
use zbus::Connection;

use super::control::{self, Signalled};
use super::objects::{Asker, Call, REQUEST_INTERFACE, retire};
use super::serve::{self, Seat};
use crate::callers::CallerSource;
use crate::env::{Bus, Env, WallClock};
use crate::intents::{IntentsLinks, IntentsService};
use crate::owner;
use crate::service::ServiceIo;

/// How long after a "from the terminal" answer the grants are read again, so intentd has stored
/// the grant the answer made.
const GRANT_SETTLE: Duration = Duration::from_millis(400);

/// What the loop reaches outside the process.
pub(crate) struct Reach {
    session: Bus,
    clock: Arc<dyn WallClock>,
    proc_root: PathBuf,
    callers: Arc<dyn CallerSource>,
}

impl Reach {
    pub(crate) fn of(env: &Env) -> Reach {
        Reach {
            session: env.buses.session.clone(),
            clock: Arc::clone(&env.clock),
            proc_root: env.proc_root.clone(),
            callers: Arc::clone(&env.callers),
        }
    }
}

/// What the loop's own tasks tell it.
enum Inner {
    Expired(Serial),
    Reload,
    AskerGone,
    Grants(Result<Vec<ActionRef>, String>),
    Resumed(Result<(), String>),
    LimitUndone(StandingGrantId, Result<(), String>),
    Signal(Signalled),
}

pub(crate) async fn run(reach: Reach, links: IntentsLinks, io: ServiceIo<IntentsService>) {
    let ServiceIo {
        state: tx,
        mut commands,
    } = io;
    loop {
        match commands.recv().await {
            Some(IntentsCommand::Serve) => break,
            // Nothing is answered, revoked or read before the daemon says to serve.
            Some(_) => {}
            None => return,
        }
    }
    let (calls_tx, mut calls) = mpsc::unbounded_channel();
    let (inner_tx, mut inner) = mpsc::unbounded_channel();
    let seat = match serve::take(
        &reach.session,
        reach.callers.as_ref(),
        calls_tx,
        links.calls,
        links.menus,
        links.shots,
        reach.proc_root,
    )
    .await
    {
        Ok(seat) => seat,
        Err(trouble) => {
            tx.send_modify(|state| state.link = IntentsLink::Unavailable(trouble));
            // Held until the service is dropped, so its handle keeps answering sends.
            while commands.recv().await.is_some() {}
            return;
        }
    };
    let Seat { connection } = seat;
    links.seat.publish(connection.clone());
    let mut live = Live {
        state: Intents::default(),
        tx,
        connection: connection.clone(),
        clock: reach.clock,
        askers: HashMap::new(),
        inner: inner_tx,
    };
    live.watch_intentd();
    live.watch_signals();
    live.feed(IntentsEvent::Link(IntentsLink::Serving));
    live.reload();
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command),
                None => break,
            },
            Some(call) = calls.recv() => live.call(call),
            Some(event) = inner.recv() => live.inner(event),
        }
    }
    live.feed(IntentsEvent::Link(IntentsLink::Passive));
    links.seat.withdraw();
}

struct Live {
    state: Intents,
    tx: watch::Sender<Intents>,
    connection: Connection,
    clock: Arc<dyn WallClock>,
    askers: HashMap<Serial, Asker>,
    inner: mpsc::UnboundedSender<Inner>,
}

impl Live {
    /// Step the state by `event`, publish it when it changed, and carry out what it asks.
    fn feed(&mut self, event: IntentsEvent) {
        let (next, effects) = step(self.state.clone(), event);
        if next != self.state {
            self.tx.send_replace(next.clone());
        }
        self.state = next;
        for effect in effects {
            self.carry_out(effect);
        }
    }

    fn carry_out(&mut self, effect: IntentsEffect) {
        match effect {
            IntentsEffect::Sheet(SheetEffect::Respond(serial, reply)) => {
                self.respond(serial, reply)
            }
            IntentsEffect::Sheet(SheetEffect::Expire { serial, after }) => {
                let wait = Duration::from_secs(u64::from(after.0));
                self.later(wait, Inner::Expired(serial));
            }
            IntentsEffect::ReadGrantsSoon => self.later(GRANT_SETTLE, Inner::Reload),
        }
    }

    /// Send `event` to the loop after `wait`.
    fn later(&self, wait: Duration, event: Inner) {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            tokio::time::sleep(wait).await;
            let _ = inner.send(event);
        });
    }

    /// `Response(code, body)` to the asker alone, then the request object goes.
    fn respond(&mut self, serial: Serial, reply: Reply) {
        let Some(asker) = self.askers.remove(&serial) else {
            return;
        };
        let (code, body) = match reply {
            Reply::Answer(answer) => (0u32, serde_json::to_string(&answer).unwrap_or_default()),
            Reply::Withdrawn => (1u32, String::new()),
        };
        let connection = self.connection.clone();
        tokio::spawn(async move {
            let sent = connection
                .emit_signal(
                    Some(asker.to.as_str()),
                    asker.path.as_ref(),
                    REQUEST_INTERFACE,
                    "Response",
                    &(code, body),
                )
                .await;
            if let Err(error) = sent {
                eprintln!("sill: intents: answering the sheet's asker: {error}");
            }
            retire(&connection, &asker.path).await;
        });
    }

    fn command(&mut self, command: IntentsCommand) {
        match command {
            IntentsCommand::Serve => {}
            IntentsCommand::Answer { id, choice } => self.answer(id, choice),
            IntentsCommand::RevokeGrant(grant) => self.revoke(grant),
            IntentsCommand::UndoLimit(id) => self.undo_limit(id),
            IntentsCommand::ResumeTerminal => self.resume(),
            IntentsCommand::RefreshGrants => self.reload(),
        }
    }

    fn answer(&mut self, id: docket_core::ConfirmId, choice: SheetChoice) {
        let at = UnixSeconds(self.clock.now().timestamp().as_second());
        self.feed(IntentsEvent::Sheet(SheetEvent::Answered { id, choice, at }));
    }

    fn call(&mut self, call: Call) {
        match call {
            Call::Asked {
                serial,
                request,
                asker,
            } => {
                self.askers.insert(serial, asker);
                self.feed(IntentsEvent::Sheet(SheetEvent::Asked(serial, request)));
            }
            Call::Cancel(id) => self.feed(IntentsEvent::Sheet(SheetEvent::Withdrawn(id))),
            Call::Closed(serial) => {
                let waiting = self
                    .state
                    .sheets
                    .queue
                    .iter()
                    .find(|pending| pending.serial == serial)
                    .map(|pending| pending.request.id.clone());
                if let Some(id) = waiting {
                    self.feed(IntentsEvent::Sheet(SheetEvent::Withdrawn(id)));
                }
            }
        }
    }

    fn inner(&mut self, event: Inner) {
        match event {
            Inner::Expired(serial) => self.feed(IntentsEvent::Sheet(SheetEvent::Expired(serial))),
            Inner::Reload => self.reload(),
            Inner::AskerGone => self.asker_gone(),
            Inner::Grants(Ok(grants)) => {
                self.feed(IntentsEvent::Terminal(TerminalEvent::Loaded(grants)));
            }
            Inner::Grants(Err(why)) => {
                eprintln!("sill: intents: reading the terminal's grants: {why}");
                self.feed(IntentsEvent::Terminal(TerminalEvent::Unreachable));
            }
            Inner::Resumed(Ok(())) => {
                self.feed(IntentsEvent::Terminal(TerminalEvent::Breaker(
                    Breaker::Running,
                )));
            }
            Inner::Resumed(Err(why)) => eprintln!("sill: intents: resuming the terminal: {why}"),
            Inner::LimitUndone(id, Ok(())) => {
                self.feed(IntentsEvent::Sheet(SheetEvent::LimitUndone(id)));
            }
            // The sheet keeps its line and its Undo: the entry is still there.
            Inner::LimitUndone(_, Err(why)) => {
                eprintln!("sill: intents: undoing a limit: {why}");
            }
            Inner::Signal(Signalled::Tripped) => {
                self.feed(IntentsEvent::Terminal(TerminalEvent::Breaker(
                    Breaker::Tripped,
                )));
            }
            Inner::Signal(Signalled::Resumed) => {
                self.feed(IntentsEvent::Terminal(TerminalEvent::Breaker(
                    Breaker::Running,
                )));
            }
        }
    }

    /// intentd left: its questions have no one waiting; their request objects go with them.
    fn asker_gone(&mut self) {
        self.feed(IntentsEvent::Sheet(SheetEvent::AskerGone));
        let connection = self.connection.clone();
        let gone: Vec<Asker> = self.askers.drain().map(|(_, asker)| asker).collect();
        tokio::spawn(async move {
            for asker in gone {
                retire(&connection, &asker.path).await;
            }
        });
    }

    /// Read the standing grants.
    fn reload(&self) {
        let (connection, inner) = (self.connection.clone(), self.inner.clone());
        tokio::spawn(async move {
            let _ = inner.send(Inner::Grants(control::grants(&connection).await));
        });
    }

    /// Withdraw `grant`: gone from the list now, read again once intentd has answered (a refusal
    /// brings it back).
    fn revoke(&mut self, grant: ActionRef) {
        self.feed(IntentsEvent::Terminal(TerminalEvent::Revoked(
            grant.clone(),
        )));
        let (connection, inner) = (self.connection.clone(), self.inner.clone());
        tokio::spawn(async move {
            if let Err(why) = control::revoke(&connection, &grant).await {
                eprintln!("sill: intents: revoking a terminal grant: {why}");
            }
            let _ = inner.send(Inner::Reload);
        });
    }

    /// Take away the limit `id`; the sheets drop their line once intentd has said it is gone.
    fn undo_limit(&self, id: StandingGrantId) {
        let (connection, inner) = (self.connection.clone(), self.inner.clone());
        tokio::spawn(async move {
            let done = control::undo_limit(&connection, &id).await;
            let _ = inner.send(Inner::LimitUndone(id, done));
        });
    }

    fn resume(&self) {
        let (connection, inner) = (self.connection.clone(), self.inner.clone());
        tokio::spawn(async move {
            let _ = inner.send(Inner::Resumed(control::resume(&connection).await));
        });
    }

    /// Follow intentd on the bus: read the grants when it arrives, drop its questions when it
    /// leaves. Costs nothing while it is absent (`NameOwnerChanged`, no timer).
    fn watch_intentd(&self) {
        let (connection, inner) = (self.connection.clone(), self.inner.clone());
        tokio::spawn(async move {
            loop {
                owner::appeared(&connection, INTENTS_BUS).await;
                if inner.send(Inner::Reload).is_err() {
                    return;
                }
                owner::changed(&connection, INTENTS_BUS).await;
                if inner.send(Inner::AskerGone).is_err() {
                    return;
                }
            }
        });
    }

    /// The control's signals into the loop.
    fn watch_signals(&self) {
        let (connection, inner) = (self.connection.clone(), self.inner.clone());
        tokio::spawn(async move {
            let (tx, mut rx) = mpsc::unbounded_channel();
            tokio::spawn({
                let connection = connection.clone();
                async move { control::forward(&connection, tx).await }
            });
            while let Some(signalled) = rx.recv().await {
                if inner.send(Inner::Signal(signalled)).is_err() {
                    return;
                }
            }
        });
    }
}
