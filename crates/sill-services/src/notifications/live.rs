//! The live loop: connect to the session bus it is given, follow
//! who holds `org.freedesktop.Notifications`, own it only while the policy says `Own` (never
//! replacing a holder: `AllowReplacement | DoNotQueue`, and ask again when the name falls
//! free), serve [`super::dbus::Server`] while owned, load the history, and run every call and
//! command through [`super::step`], carrying out its effects (signals,
//! `history_file::save`; a `Sound` is dropped, as sill plays no sounds yet) and
//! publishing the result.
//!
//! Nothing here may ever touch the real session bus from a test: tests pass
//! `BusRoute::Address` of a private `dbus-daemon` (`tests/support/private_bus.rs`).

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{mpsc, watch};

use super::content::{Notification, Posting};
use super::dbus::{Call, CloseAnswer};
use super::history_file;
use super::ids::{NotificationId, UnixMs};
use super::name_watch::Bus;
use super::policy::NotificationsPolicy;
use super::reduce::{NotesState, NotificationsEffect, NotificationsEvent, assign_id};
use super::rules;
use super::state::{NameHolder, ServerMode, ServerStatus};
use super::{NotificationsCommand, step};
use crate::env::{Bus as BusRoute, WallClock};
use crate::stream::next_opt;

/// Run the service until every control is dropped.
pub(super) async fn run(
    route: BusRoute,
    file: PathBuf,
    clock: Arc<dyn WallClock>,
    published: watch::Sender<NotesState>,
    mut commands: mpsc::UnboundedReceiver<NotificationsCommand>,
) {
    let mut core = Core {
        state: NotesState::default(),
        policy: NotificationsPolicy::default(),
        file,
        published,
        clock,
    };
    core.apply(
        NotificationsEvent::Loaded(history_file::load(&core.file)),
        None,
    )
    .await;
    // Held for the whole run, so `calls` never closes (a closed channel would wake the select
    // forever) even when there is no bus.
    let (calls_tx, mut calls) = mpsc::unbounded_channel();
    let (bus, mut owner_changes) = match Bus::connect(&route, calls_tx.clone()).await {
        Some((bus, changes)) => (Some(bus), changes),
        None => (None, None),
    };
    core.settle(bus.as_ref()).await;
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => core.command(command, bus.as_ref()).await,
                None => break,
            },
            Some(call) = calls.recv() => core.call(call, bus.as_ref()).await,
            Some(_) = next_opt(&mut owner_changes) => core.settle(bus.as_ref()).await,
        }
    }
    if let Some(bus) = &bus
        && core.state.live.server.holder == NameHolder::Sill
    {
        bus.release().await;
    }
    drop(calls_tx);
}

/// The loop's state and where it publishes.
struct Core {
    state: NotesState,
    policy: NotificationsPolicy,
    /// Where the history is kept.
    file: PathBuf,
    published: watch::Sender<NotesState>,
    /// What stamps a posting's arrival.
    clock: Arc<dyn WallClock>,
}

impl Core {
    async fn command(&mut self, command: NotificationsCommand, bus: Option<&Bus>) {
        let event = match command {
            NotificationsCommand::Policy(policy) => {
                self.policy = policy;
                self.settle(bus).await;
                return;
            }
            NotificationsCommand::Post(posting) => {
                self.post(*posting, bus).await;
                return;
            }
            NotificationsCommand::Dismiss(id) => NotificationsEvent::Dismissed(id),
            NotificationsCommand::SwipeAway(id) => NotificationsEvent::SwipedAway(id),
            NotificationsCommand::DismissAll => NotificationsEvent::DismissedAll,
            NotificationsCommand::BannerTimedOut(id) => NotificationsEvent::BannerTimedOut(id),
            NotificationsCommand::InvokeAction { id, action, token } => {
                NotificationsEvent::Invoked { id, action, token }
            }
            NotificationsCommand::ClearHistory(clear) => NotificationsEvent::HistoryCleared(clear),
        };
        self.apply(event, bus).await;
    }

    async fn call(&mut self, call: Call, bus: Option<&Bus>) {
        match call {
            Call::Notify { posting, reply } => {
                let id = self.post(*posting, bus).await;
                let _ = reply.send(id);
            }
            Call::Close { id, reply } => {
                let answer = if rules::holds(&self.state, id) {
                    self.apply(NotificationsEvent::ClosedByApp(id), bus).await;
                    CloseAnswer::Closed
                } else {
                    CloseAnswer::NotHeld
                };
                let _ = reply.send(answer);
            }
        }
    }

    /// Give `posting` its id and time, and post it.
    async fn post(&mut self, posting: Posting, bus: Option<&Bus>) -> NotificationId {
        let id = assign_id(&self.state, posting.replaces);
        let note = Notification::posted(id, unix_ms(&self.clock.now()), posting);
        self.apply(NotificationsEvent::Posted(Box::new(note)), bus)
            .await;
        id
    }

    /// Ask for or give back the name as the policy says, and publish who holds it.
    async fn settle(&mut self, bus: Option<&Bus>) {
        let mode = self.policy.server;
        let holder = match bus {
            Some(bus) => bus.settle(mode).await,
            None => NameHolder::NoBus,
        };
        self.apply(
            NotificationsEvent::Server(ServerStatus { mode, holder }),
            bus,
        )
        .await;
    }

    async fn apply(&mut self, event: NotificationsEvent, bus: Option<&Bus>) {
        let state = std::mem::take(&mut self.state);
        let (state, effects) = step(state, event, &self.policy);
        self.state = state;
        publish(&self.published, &self.state);
        for effect in effects {
            self.perform(effect, bus).await;
        }
    }

    async fn perform(&self, effect: NotificationsEffect, bus: Option<&Bus>) {
        // Signals only while sill holds the name: a passive sill's ids are not the holder's.
        let speaking = match (bus, &self.state.live.server) {
            (
                Some(bus),
                ServerStatus {
                    holder: NameHolder::Sill,
                    mode: ServerMode::Own,
                },
            ) => Some(bus),
            _ => None,
        };
        match (effect, speaking) {
            (NotificationsEffect::Closed { id, reason }, Some(bus)) => bus.closed(id, reason).await,
            (NotificationsEffect::Invoked { id, action, token }, Some(bus)) => {
                bus.invoked(id, &action, &token).await;
            }
            (NotificationsEffect::SaveHistory, _) => self.save(),
            (
                NotificationsEffect::Closed { .. }
                | NotificationsEffect::Invoked { .. }
                | NotificationsEffect::Sound(_),
                _,
            ) => {}
        }
    }

    fn save(&self) {
        if let Err(error) = history_file::save(&self.file, &history_file::for_disk(&self.state)) {
            eprintln!("sill: notifications: saving the history: {error}");
        }
    }
}

/// Send `value` when it differs from what `tx` holds.
fn publish<T: Clone + PartialEq>(tx: &watch::Sender<T>, value: &T) {
    tx.send_if_modified(|held| {
        if held == value {
            return false;
        }
        value.clone_into(held);
        true
    });
}

/// `at` in milliseconds since the Unix epoch.
fn unix_ms(at: &jiff::Zoned) -> UnixMs {
    UnixMs(at.timestamp().as_millisecond())
}
