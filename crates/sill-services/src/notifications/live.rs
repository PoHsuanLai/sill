//! The live loop (lane a, FINDINGS "The M6 fill-in"): connect to the bus `setup` names, follow
//! who holds `org.freedesktop.Notifications`, own it only while the policy says `Own` (never
//! replacing a holder: `AllowReplacement | DoNotQueue`, and ask again when the name falls
//! free), serve [`super::dbus::Server`] while owned, load the history, and run every call and
//! command through [`super::reduce_notifications`], carrying out its effects (signals,
//! `history_file::save`; a `Sound` is dropped until the sound service is real, F271) and
//! publishing both receivers.
//!
//! Nothing here may ever touch the real session bus from a test: tests pass
//! `BusChoice::Address` of a private `dbus-daemon` (`tests/support/private_bus.rs`).

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::{mpsc, watch};

use super::content::{Notification, Posting};
use super::dbus::{Call, CloseAnswer};
use super::history_file;
use super::ids::{NotificationId, UnixMs};
use super::name_watch::{Bus, next_opt};
use super::policy::NotificationsPolicy;
use super::reduce::{NotesState, NotificationsEffect, NotificationsEvent, assign_id};
use super::rules;
use super::state::{History, NameHolder, Notifications, ServerMode, ServerStatus};
use super::{HistoryPlace, NotificationsCommand, Setup, reduce_notifications};

/// Run the service until every control is dropped.
pub(super) async fn run(
    setup: Setup,
    live: watch::Sender<Notifications>,
    history: watch::Sender<History>,
    mut commands: mpsc::UnboundedReceiver<NotificationsCommand>,
) {
    let mut core = Core {
        state: NotesState::default(),
        policy: NotificationsPolicy::default(),
        file: file_of(&setup.history),
        live,
        history,
    };
    if let Some(path) = &core.file {
        core.apply(NotificationsEvent::Loaded(history_file::load(path)), None)
            .await;
    }
    // Held for the whole run, so `calls` never closes (a closed channel would wake the select
    // forever) even when there is no bus.
    let (calls_tx, mut calls) = mpsc::unbounded_channel();
    let (bus, mut owner_changes) = match Bus::connect(&setup.bus, calls_tx.clone()).await {
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

/// The file the history lives in, if any.
fn file_of(place: &HistoryPlace) -> Option<PathBuf> {
    match place {
        HistoryPlace::State => history_file::default_path(),
        HistoryPlace::At(path) => Some(path.clone()),
        HistoryPlace::NotKept => None,
    }
}

/// The loop's state and where it publishes.
struct Core {
    state: NotesState,
    policy: NotificationsPolicy,
    file: Option<PathBuf>,
    live: watch::Sender<Notifications>,
    history: watch::Sender<History>,
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
        let note = Notification::posted(id, now(), posting);
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
        let (state, effects) = reduce_notifications(state, event, &self.policy);
        self.state = state;
        publish(&self.live, &self.state.live);
        publish(&self.history, &self.state.history);
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
        let Some(path) = &self.file else {
            return;
        };
        if let Err(error) = history_file::save(path, &history_file::for_disk(&self.state)) {
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

fn now() -> UnixMs {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    UnixMs(i64::try_from(ms).unwrap_or(i64::MAX))
}
