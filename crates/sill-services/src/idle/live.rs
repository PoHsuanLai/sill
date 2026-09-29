//! The idle service's loop:
//!
//! - passive (no Wayland connection, no bus name, no logind call) until the first `Configure`
//!   or `Use`, and again whenever the policy's mode is `Off`;
//! - `Configure` is resolved on the blocking pool ([`super::resolve`]) and again every
//!   [`RESOLVE_EVERY_S`] seconds; `Use` is taken as it is;
//! - while `Own`: the Wayland thread ([`super::wl`]) on `Reach.display`, logind on
//!   `Reach.system` ([`super::logind`]; `BlockInhibited` polled every [`POLL_S`] seconds) and
//!   `org.freedesktop.ScreenSaver` on `Reach.session` ([`super::screensaver`]);
//! - follows `IdleLinks.lock` and `IdleLinks.battery` into `IdleEvent::{Lock, Power}`;
//! - steps [`step`] and performs its effects in order, publishing [`Idle`].

use std::time::{Duration, Instant};

use logind_zbus::manager::PrepareForSleepStream;
use tokio::sync::{mpsc, watch};
use tokio::time::MissedTickBehavior;
use zbus::fdo::NameOwnerChangedStream;

use super::inhibit::{InhibitMsg, Inhibitors, LogindIdle, inhibit_step, inhibition};
use super::logind::{self as idle_logind, IdleLogind};
use super::model::{Idle, IdleCommand, IdleLinks, Reach};
use super::policy::{IdleConfig, IdleMode, LockView, power_source};
use super::reduce::{IdleEffect, IdleEvent, IdleState, LockSeen, step};
use super::resolve::{RESOLVE_EVERY_S, resolve};
use super::screensaver::{self, Call, Served};
use super::wl::{self, Epoch, Heard, Order, Tell, Wire, WlEvent};
use crate::auth::AuthCommand;
use crate::idle::grace::ticket;
use crate::locker::{Lock, LockPhase, LockerCommand};
use crate::stream::next_opt;
use sill_model::units::Stamp;

/// How often logind's `BlockInhibited` is read.
const POLL_S: u64 = 5;

/// The parts that exist only while idle is `Own`.
#[derive(Debug, Default)]
struct Held {
    wire: Option<Wire>,
    logind: Option<IdleLogind>,
    sleep: Option<PrepareForSleepStream>,
    served: Option<Served>,
    departures: Option<NameOwnerChangedStream>,
}

/// Everything the loop owns.
struct Live {
    setup: Reach,
    links: IdleLinks,
    tx: watch::Sender<Idle>,
    epoch: Instant,
    state: IdleState,
    /// The last `Configure`, resolved again on the timer.
    config: Option<IdleConfig>,
    held: Held,
    wire_epoch: Epoch,
    heard: mpsc::UnboundedSender<Heard>,
    calls: mpsc::UnboundedSender<Call>,
    inhibitors: Inhibitors,
    logind_idle: LogindIdle,
}

pub(super) async fn run(
    setup: Reach,
    links: IdleLinks,
    mut commands: mpsc::UnboundedReceiver<IdleCommand>,
    tx: watch::Sender<Idle>,
) {
    let (heard, mut from_wire) = mpsc::unbounded_channel();
    let (calls, mut from_bus) = mpsc::unbounded_channel();
    let mut lock = links.lock.clone();
    let mut battery = links.battery.clone();
    let mut live = Live {
        setup,
        links,
        tx,
        epoch: Instant::now(),
        state: IdleState::default(),
        config: None,
        held: Held::default(),
        wire_epoch: Epoch(0),
        heard,
        calls,
        inhibitors: Inhibitors::default(),
        logind_idle: LogindIdle::Free,
    };
    live.follow_lock(&lock.borrow_and_update());
    live.follow_power(&battery.borrow_and_update());
    let mut poll = tokio::time::interval(Duration::from_secs(POLL_S));
    poll.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut ticks: u64 = 0;
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command).await,
                None => break,
            },
            Some(heard) = from_wire.recv() => live.wire_said(heard).await,
            Some(call) = from_bus.recv() => live.bus_call(call).await,
            Ok(()) = lock.changed() => {
                let seen = *lock.borrow_and_update();
                live.follow_lock(&seen);
                live.feed_lock(&seen).await;
            }
            Ok(()) = battery.changed() => {
                let battery = battery.borrow_and_update().clone();
                let power = power_source(&battery);
                live.feed(IdleEvent::Power(power)).await;
            }
            Some(signal) = next_opt(&mut live.held.sleep) => {
                let now = live.now();
                match signal.args() {
                    Ok(args) if *args.start() => live.feed(IdleEvent::SleepStarting(now)).await,
                    Ok(_) => live.feed(IdleEvent::Woke(now)).await,
                    Err(_) => {}
                }
            }
            Some(change) = next_opt(&mut live.held.departures) => {
                if let Ok(args) = change.args()
                    && args.new_owner().is_none()
                    && args.name().starts_with(':')
                {
                    let peer = super::inhibit::BusPeer(args.name().to_string());
                    live.inhibit(InhibitMsg::PeerGone(peer)).await;
                }
            }
            _ = poll.tick() => {
                ticks += 1;
                live.poll_logind().await;
                if ticks.is_multiple_of(RESOLVE_EVERY_S / POLL_S) {
                    live.resolve_again().await;
                }
            }
        }
    }
    live.feed_policy_off().await;
}

impl Live {
    fn now(&self) -> Stamp {
        Stamp(u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX))
    }

    async fn command(&mut self, command: IdleCommand) {
        let policy = match command {
            IdleCommand::Configure(config) => {
                self.config = Some(config);
                resolved(config, &self.setup).await
            }
            IdleCommand::Use(policy) => {
                self.config = None;
                policy
            }
            IdleCommand::FakeWake => {
                let now = self.now();
                self.feed(IdleEvent::Resumed(now)).await;
                self.order(Order::Renew(self.state.held.clone()));
                return;
            }
        };
        self.use_policy(policy).await;
    }

    async fn resolve_again(&mut self) {
        let Some(config) = self.config else {
            return;
        };
        let policy = resolved(config, &self.setup).await;
        if policy != self.state.policy {
            self.use_policy(policy).await;
        }
    }

    /// Start what `Own` needs before the machine sees the policy, and stop it after `Off`.
    async fn use_policy(&mut self, policy: super::IdlePolicy) {
        if policy.mode != self.state.policy.mode {
            eprintln!("sill: idle: {:?}", policy.mode);
        }
        match policy.mode {
            IdleMode::Own => {
                self.hold().await;
                self.feed(IdleEvent::Policy(policy)).await;
            }
            IdleMode::Off(_) => {
                self.feed(IdleEvent::Policy(policy)).await;
                self.release();
            }
        }
    }

    async fn feed_policy_off(&mut self) {
        let off = super::IdlePolicy {
            mode: IdleMode::Off(super::OffReason::NoPolicy),
            ..self.state.policy
        };
        self.feed(IdleEvent::Policy(off)).await;
        self.release();
    }

    /// Connect the Wayland thread and the buses, if not already.
    async fn hold(&mut self) {
        if self.held.wire.is_none() {
            self.wire_epoch = Epoch(self.wire_epoch.0 + 1);
            let tell = Tell::new(self.wire_epoch, self.heard.clone());
            match wl::start(self.setup.display.clone(), tell) {
                Ok(wire) => self.held.wire = Some(wire),
                Err(error) => eprintln!("sill: idle: no thread: {error}"),
            }
            // A fresh connection holds nothing: the machine's set must be sent again.
            self.state.held.clear();
        }
        if self.held.logind.is_none() {
            self.held.logind = idle_logind::connect(&self.setup.system).await;
            self.held.sleep = self.held.logind.as_mut().and_then(|l| l.sleep.take());
        }
        if self.held.served.is_none() {
            let idle = self.tx.subscribe();
            let calls = self.calls.clone();
            self.held.served = screensaver::serve(&self.setup.session, calls, idle).await;
            self.held.departures = self.held.served.as_mut().and_then(|s| s.departures.take());
        }
    }

    /// Drop the thread and the buses (the name goes back with its connection).
    fn release(&mut self) {
        self.held = Held::default();
        self.state.held.clear();
        self.inhibitors = Inhibitors::default();
        self.logind_idle = LogindIdle::Free;
    }

    async fn wire_said(&mut self, heard: Heard) {
        if heard.epoch != self.wire_epoch {
            return;
        }
        let now = self.now();
        match heard.event {
            WlEvent::Bound(protocols) => eprintln!("sill: idle: bound {protocols:?}"),
            WlEvent::Idled(stage) => self.feed(IdleEvent::Idled(stage, now)).await,
            WlEvent::Resumed => self.feed(IdleEvent::Resumed(now)).await,
            WlEvent::Gone => {
                self.held.wire = None;
                self.state.held.clear();
            }
        }
    }

    async fn bus_call(&mut self, call: Call) {
        let reply = self.inhibit(call.msg).await;
        let _ = call.reply.send(reply);
    }

    async fn inhibit(&mut self, msg: InhibitMsg) -> super::InhibitReply {
        if let InhibitMsg::Inhibit { app, reason, .. } = &msg {
            eprintln!("sill: idle: inhibited by {app:?} ({reason:?})");
        }
        let (next, reply) = inhibit_step(std::mem::take(&mut self.inhibitors), msg);
        self.inhibitors = next;
        self.feed_inhibition().await;
        reply
    }

    async fn poll_logind(&mut self) {
        let Some(logind) = &self.held.logind else {
            return;
        };
        let blocked = logind.blocked().await;
        if blocked != self.logind_idle {
            self.logind_idle = blocked;
            self.feed_inhibition().await;
        }
    }

    async fn feed_inhibition(&mut self) {
        let now = inhibition(&self.inhibitors, self.logind_idle);
        if now != self.state.inhibition {
            self.feed(IdleEvent::Inhibition(now)).await;
        } else {
            self.publish();
        }
    }

    /// Keep the lock and the power in the state while passive (no effects then).
    fn follow_lock(&mut self, lock: &Lock) {
        self.state.lock = seen_of(lock);
    }

    fn follow_power(&mut self, battery: &sill_model::battery::Battery) {
        self.state.power = power_source(battery);
    }

    async fn feed_lock(&mut self, lock: &Lock) {
        self.feed(IdleEvent::Lock(seen_of(lock))).await;
    }

    /// Step the machine and perform its effects in order.
    async fn feed(&mut self, event: IdleEvent) {
        let (next, effects) = step(std::mem::take(&mut self.state), event);
        self.state = next;
        for effect in effects {
            self.perform(effect).await;
        }
        self.publish();
    }

    async fn perform(&mut self, effect: IdleEffect) {
        match effect {
            IdleEffect::Watch(watches) => self.order(Order::Watch(watches)),
            IdleEffect::Displays(power) => {
                eprintln!("sill: idle: displays {power:?}");
                self.order(Order::Displays(power));
            }
            // The dim overlay (`sill-surfaces` `idle_dim`, mapped by the daemon's
            // `idle_dim` follower) reads `Idle.dim`, published after the step.
            IdleEffect::Dim(level) => eprintln!("sill: idle: dim {level:?}"),
            IdleEffect::Undim => eprintln!("sill: idle: undim"),
            IdleEffect::Lock => {
                eprintln!("sill: idle: lock at screen off");
                let _ = self.links.locker.send(LockerCommand::IdleLock);
            }
            IdleEffect::Grace(case) => {
                let verdict = super::grace_verdict(&case);
                eprintln!(
                    "sill: idle: grace {verdict:?} (woke after {} ms)",
                    case.woke.0
                );
                if let Some(ticket) = ticket(&case) {
                    let _ = self.links.auth.send(AuthCommand::Grace(ticket));
                }
            }
            IdleEffect::Suspend => match &self.held.logind {
                Some(logind) => {
                    eprintln!("sill: idle: suspend");
                    logind.suspend().await;
                }
                None => eprintln!("sill: idle: no logind to suspend"),
            },
            IdleEffect::IdleHint(hint) => {
                if let Some(logind) = &self.held.logind {
                    logind.idle_hint(hint).await;
                }
            }
        }
    }

    fn order(&self, order: Order) {
        if let Some(wire) = &self.held.wire {
            wire.order(order);
        }
    }

    fn publish(&self) {
        let idle = Idle {
            mode: self.state.policy.mode,
            phase: self.state.phase,
            dim: self.state.dim,
            power: self.state.power,
            inhibition: self.state.inhibition,
            inhibits: u32::try_from(self.inhibitors.held.len()).unwrap_or(u32::MAX),
            logind: self.logind_idle,
        };
        self.tx.send_if_modified(|current| {
            let changed = *current != idle;
            *current = idle;
            changed
        });
    }
}

/// `config` resolved on the blocking pool; `Off` if that fails.
async fn resolved(config: IdleConfig, reach: &Reach) -> super::IdlePolicy {
    let dir = reach.config.clone();
    let processes = reach.processes.clone();
    tokio::task::spawn_blocking(move || resolve(&config, &dir, processes.as_ref()))
        .await
        .unwrap_or(super::IdlePolicy {
            mode: IdleMode::Off(super::OffReason::NoPolicy),
            ..super::PASSIVE
        })
}

/// The lock as idle reads it: `Locking` counts as locked.
fn seen_of(lock: &Lock) -> LockSeen {
    let view = match lock.phase {
        LockPhase::Unlocked => LockView::Unlocked,
        LockPhase::Locking | LockPhase::Locked => LockView::Locked,
    };
    LockSeen {
        view,
        cause: lock.cause,
    }
}
