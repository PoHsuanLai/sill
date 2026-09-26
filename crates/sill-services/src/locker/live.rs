//! The locker service's loop: turns commands, logind's signals, the locker's words and the clock
//! into [`LockerEvent`]s, steps the machine, performs its effects and publishes [`Lock`].

use std::time::Instant;

use tokio::sync::{mpsc, watch};
use zbus::zvariant::OwnedFd;

use super::child::{self, Generation, Heard, Word};
use super::choice::{Host, LockerChoice, LockerMode, SessionManager, resolve};
use super::logind::{self, Logind, next_opt};
use super::machine::{Cause, Locker, LockerEffect, LockerEvent, next_deadline, step};
use super::{Lock, LockDrawer, LockerCommand, ShellLockEvent};
use crate::programs::{self, Pid};
use crate::units::Stamp;

/// Everything the loop owns besides the machine.
struct Live {
    tx: watch::Sender<Lock>,
    machine: Locker,
    mode: LockerMode,
    epoch: Instant,
    logind: Logind,
    inhibitor: Option<OwnedFd>,
    generation: Generation,
    pid: Option<Pid>,
    words: mpsc::UnboundedSender<Word>,
}

/// Runs until `commands` closes. Inert (`LockerMode::Off`) until the first `Configure`/`Use`.
pub(super) async fn run(
    tx: watch::Sender<Lock>,
    mut commands: mpsc::UnboundedReceiver<LockerCommand>,
) {
    let (words, mut heard) = mpsc::unbounded_channel();
    let mut live = Live {
        tx,
        machine: Locker::default(),
        mode: LockerMode::Off,
        epoch: Instant::now(),
        logind: logind::connect().await,
        inhibitor: None,
        generation: Generation::default(),
        pid: None,
        words,
    };
    loop {
        let deadline = next_deadline(&live.machine).map(|at| live.instant(at));
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command).await,
                None => break,
            },
            Some(word) = heard.recv() => live.word(word).await,
            Some(signal) = next_opt(&mut live.logind.sleep) => {
                let now = live.now();
                match signal.args() {
                    Ok(args) if *args.start() => live.feed(LockerEvent::SleepStarting(now)).await,
                    Ok(_) => live.feed(LockerEvent::Woke).await,
                    Err(_) => {}
                }
            }
            Some(_) = next_opt(&mut live.logind.lock) => {
                let now = live.now();
                live.feed(LockerEvent::Requested(Cause::Logind, now)).await;
            }
            () = sleep_until(deadline) => {
                let now = live.now();
                live.feed(LockerEvent::Tick(now)).await;
            }
        }
    }
}

async fn sleep_until(deadline: Option<Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at.into()).await,
        None => std::future::pending().await,
    }
}

impl Live {
    fn now(&self) -> Stamp {
        Stamp(u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX))
    }

    fn instant(&self, stamp: Stamp) -> Instant {
        self.epoch + std::time::Duration::from_millis(stamp.0)
    }

    async fn command(&mut self, command: LockerCommand) {
        match command {
            LockerCommand::Configure(choice) => {
                let mode = tokio::task::spawn_blocking(move || detect(&choice))
                    .await
                    .unwrap_or(LockerMode::Off);
                self.use_mode(mode).await;
            }
            LockerCommand::Use(mode) => self.use_mode(mode).await,
            LockerCommand::Lock => {
                let now = self.now();
                self.feed(LockerEvent::Requested(Cause::Action, now)).await;
            }
            LockerCommand::Shell(event) => {
                let event = match event {
                    ShellLockEvent::Locked => LockerEvent::Reported,
                    ShellLockEvent::Finished | ShellLockEvent::Refused => LockerEvent::Exited,
                };
                self.feed(event).await;
            }
        }
    }

    async fn use_mode(&mut self, mode: LockerMode) {
        if mode == self.mode {
            return;
        }
        let (runner, triggers) = mode.runner();
        self.mode = mode;
        self.feed(LockerEvent::Configured(runner, triggers)).await;
        // A locker already running (a previous sill's, or one started by hand) is adopted, so
        // the next Lock never starts a second.
        if let Some(pid) = self.running_locker() {
            self.adopt(pid).await;
        }
    }

    fn running_locker(&self) -> Option<Pid> {
        match (&self.mode, self.pid) {
            (LockerMode::Run { plan, .. }, None) => plan
                .process_name
                .as_deref()
                .and_then(|name| programs::running(name).into_iter().next()),
            _ => None,
        }
    }

    async fn adopt(&mut self, pid: Pid) {
        self.generation = Generation(self.generation.0 + 1);
        self.pid = Some(pid);
        child::watch_adopted(pid, self.generation, self.words.clone());
        self.feed(LockerEvent::Adopted).await;
    }

    async fn word(&mut self, word: Word) {
        if word.generation != self.generation {
            return;
        }
        match word.heard {
            Heard::Ready => self.feed(LockerEvent::Reported).await,
            Heard::Exited => {
                self.pid = None;
                self.feed(LockerEvent::Exited).await;
            }
        }
    }

    /// Steps the machine with `event` and performs what it asks, feeding back any failure.
    async fn feed(&mut self, event: LockerEvent) {
        let mut pending = vec![event];
        while let Some(event) = pending.pop() {
            let (next, effects) = step(self.machine, event);
            self.machine = next;
            for effect in effects {
                pending.extend(self.perform(effect).await);
            }
        }
        self.publish();
    }

    async fn perform(&mut self, effect: LockerEffect) -> Option<LockerEvent> {
        match effect {
            LockerEffect::Spawn => {
                if let Some(pid) = self.running_locker() {
                    // Started elsewhere since sill last looked: adopt it instead.
                    self.generation = Generation(self.generation.0 + 1);
                    self.pid = Some(pid);
                    child::watch_adopted(pid, self.generation, self.words.clone());
                    return Some(LockerEvent::Adopted);
                }
                let plan = match &self.mode {
                    LockerMode::Run { plan, .. } => plan,
                    // The daemon follows the published phase onto shell-host's session lock and
                    // answers with `LockerCommand::Shell` (FINDINGS F524).
                    LockerMode::Own { .. } => return None,
                    LockerMode::Off | LockerMode::Delegate => return Some(LockerEvent::Exited),
                };
                self.generation = Generation(self.generation.0 + 1);
                self.pid = child::start(plan, self.generation, self.words.clone());
                None
            }
            LockerEffect::AskLogind => {
                if let (Some(manager), Some(id)) = (&self.logind.manager, &self.logind.session_id) {
                    let _ = manager.lock_session(id).await;
                }
                None
            }
            LockerEffect::TakeInhibitor => {
                let fd = match &self.logind.manager {
                    Some(manager) => logind::inhibit(manager).await,
                    None => None,
                };
                self.inhibitor = fd;
                self.inhibitor
                    .is_none()
                    .then_some(LockerEvent::InhibitorRefused)
            }
            LockerEffect::ReleaseInhibitor => {
                self.inhibitor = None;
                None
            }
        }
    }

    fn publish(&self) {
        let drawer = match self.mode {
            LockerMode::Own { .. } => LockDrawer::Shell,
            LockerMode::Off | LockerMode::Delegate | LockerMode::Run { .. } => LockDrawer::Program,
        };
        let lock = Lock {
            phase: self.machine.phase.public(),
            pid: self.pid,
            drawer,
        };
        self.tx.send_if_modified(|current| {
            let changed = *current != lock;
            *current = lock;
            changed
        });
    }
}

/// Resolves `choice` against this session and this machine (blocking: `/proc` and `PATH`).
fn detect(choice: &LockerChoice) -> LockerMode {
    let manager = if programs::running("cosmic-session").is_empty() {
        SessionManager::Other
    } else {
        SessionManager::CosmicSession
    };
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    resolve(choice, Host::of(manager, &desktop), programs::on_path)
}
