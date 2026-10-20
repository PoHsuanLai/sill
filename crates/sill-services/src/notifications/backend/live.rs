//! The live loop: connect to the session bus it is given, follow
//! who holds `org.freedesktop.Notifications`, own it only while the policy says `Own` (never
//! replacing a holder: `DoNotQueue`, and ask again when the name falls free; once held, the
//! name cannot be taken away), serve [`super::dbus::Server`] while owned, load the history, and
//! run every call and command through [`sill_model::notifications::step`], carrying out its
//! effects (signals, and the history's write, gathered and run off the loop by [`super::save`]; a
//! `Sound` is dropped, as sill plays no sounds yet) and publishing the result. Each bus caller is
//! held to its allowance and may replace or close only what it posted ([`super::senders`]); a
//! clock tick sweeps an app's critical banners that have stood too long.
//!
//! Nothing here may ever touch the real session bus from a test: tests pass
//! `BusRoute::Address` of a private `dbus-daemon` (`sill-testkit/src/private_bus.rs`).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use sill_model::notifications::{
    ImageDirs, NameHolder, NotesState, Notification, NotificationId, NotificationsCommand,
    NotificationsEffect, NotificationsEvent, NotificationsPolicy, Posting, Said, ServerMode,
    ServerStatus, UnixMs, assign_id, for_disk, holds, step,
};
use tokio::sync::{mpsc, watch};

use super::dbus::{Call, Caller, CloseAnswer, NotifyAnswer, Sender};
use super::history_file;
use super::name_watch::Bus;
use super::save::{SAVE_AFTER, Saver};
use super::senders::{Admission, Owner, Senders};
use crate::env::{Bus as BusRoute, WallClock};
use crate::notifications::Answers;
use crate::stream::next_opt;

/// How often an app's critical banners are checked against their hold: the hold is minutes long,
/// so a minute's slack is not seen.
const SWEEP_EVERY: Duration = Duration::from_secs(60);

/// Run the service until every control is dropped.
pub(crate) async fn run(
    route: BusRoute,
    file: PathBuf,
    clock: Arc<dyn WallClock>,
    images: ImageDirs,
    answers: Answers,
    published: watch::Sender<NotesState>,
    mut commands: mpsc::UnboundedReceiver<NotificationsCommand>,
) {
    let stored = history_file::load(&file);
    let mut core = Core {
        state: NotesState::default(),
        policy: NotificationsPolicy::default(),
        published,
        clock,
        images,
        answers,
        senders: Senders::default(),
        saver: Saver::new(file, SAVE_AFTER),
    };
    core.apply(NotificationsEvent::Loaded(stored), None).await;
    // Held for the whole run, so `calls` never closes (a closed channel would wake the select
    // forever) even when there is no bus.
    let (calls_tx, mut calls) = mpsc::unbounded_channel();
    let (bus, mut owner_changes) = match Bus::connect(&route, calls_tx.clone()).await {
        Some((bus, changes)) => (Some(bus), changes),
        None => (None, None),
    };
    core.settle(bus.as_ref()).await;
    let mut sweep = tokio::time::interval(SWEEP_EVERY);
    sweep.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => core.command(command, bus.as_ref()).await,
                None => break,
            },
            Some(call) = calls.recv() => core.call(call, bus.as_ref()).await,
            Some(_) = next_opt(&mut owner_changes) => core.settle(bus.as_ref()).await,
            _ = sweep.tick() => core.sweep(bus.as_ref()).await,
            () = core.saver.ready() => core.write_history(),
        }
    }
    core.saver.finish(|| for_disk(&core.state)).await;
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
    published: watch::Sender<NotesState>,
    /// What stamps a posting's arrival.
    clock: Arc<dyn WallClock>,
    /// Where a bus caller's pictures may come from.
    images: ImageDirs,
    /// Where the person's answers are said to the shell's own services.
    answers: Answers,
    /// Who posted what, and how much each caller may still post.
    senders: Senders,
    /// When the history is next written.
    saver: Saver,
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
                let id = self.post(*posting, bus).await;
                self.senders.record(id, None);
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
            Call::Notify {
                posting,
                sender,
                caller,
                reply,
            } => {
                let answer = self.notify(*posting, sender, &caller, bus).await;
                let _ = reply.send(answer);
            }
            Call::Close { id, sender, reply } => {
                let answer = if holds(&self.state, id) && self.senders.owns(&sender, id) {
                    self.apply(NotificationsEvent::ClosedByApp(id), bus).await;
                    self.senders.forget_gone(&self.state);
                    CloseAnswer::Closed
                } else {
                    CloseAnswer::NotHeld
                };
                let _ = reply.send(answer);
            }
        }
    }

    /// A `Notify` from `sender`, counted against `caller`: refused past the caller's allowance or
    /// the shared one; it replaces only what it posted or what was posted for the same app; its
    /// pictures stay only where [`ImageDirs`] allow.
    async fn notify(
        &mut self,
        mut posting: Posting,
        sender: Sender,
        caller: &Caller,
        bus: Option<&Bus>,
    ) -> NotifyAnswer {
        let now = unix_ms(&self.clock.now());
        if self.senders.admit(caller, now) == Admission::TooMany {
            return NotifyAnswer::TooMany;
        }
        posting = posting.within(&self.images);
        let app = posting.app.key();
        posting.replaces = self.senders.replaceable(&sender, &app, posting.replaces);
        let id = self.post(posting, bus).await;
        self.senders.record(id, Some(Owner { sender, app }));
        self.senders.forget_gone(&self.state);
        NotifyAnswer::Posted(id)
    }

    /// The clock reached the next sweep.
    async fn sweep(&mut self, bus: Option<&Bus>) {
        let now = unix_ms(&self.clock.now());
        self.apply(NotificationsEvent::Swept(now), bus).await;
        self.senders.forget_gone(&self.state);
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

    async fn perform(&mut self, effect: NotificationsEffect, bus: Option<&Bus>) {
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
        match effect {
            NotificationsEffect::Closed { id, reason } => {
                self.answers.say(Said::Closed { id, reason });
                if let Some(bus) = speaking {
                    bus.closed(id, reason).await;
                }
            }
            NotificationsEffect::Invoked { id, action, token } => {
                self.answers.say(Said::Pressed {
                    id,
                    key: action.clone(),
                });
                if let Some(bus) = speaking {
                    bus.invoked(id, &action, &token, self.senders.sender_of(id))
                        .await;
                }
            }
            NotificationsEffect::SaveHistory => self.saver.request(),
            NotificationsEffect::Sound(_) => {}
        }
    }

    /// The saver's time has come: the history as it is now, written on the blocking pool.
    fn write_history(&mut self) {
        self.saver.start(for_disk(&self.state));
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
