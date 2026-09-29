//! The locker service's loop: turns commands, logind's signals, the shell's lock reports and the
//! clock into [`LockerEvent`]s, steps the machine, performs its effects and publishes [`Lock`].
//!
//! In `LockerMode::Own` the loop also keeps [`super::marker`]'s file (written on
//! `Shell(Locked)`, removed on `Shell(Unlocked | Finished | Refused)`) and, right after a mode
//! resolves to `Own`, locks at once when [`super::marker::restore_on_start`] says so.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::{mpsc, watch};
use zbus::zvariant::OwnedFd;

use super::cause::{LockCause, cause_after, cause_of};
use super::logind::{self, Logind};
use super::machine::{Cause, Locker, LockerEffect, LockerEvent, next_deadline, step};
use super::marker::{LockMarker, Restore, marker_path, restore_on_start};
use super::mode::{Host, LockerMode, SessionManager};
use super::{Lock, LockerCommand, ShellLockEvent};
use crate::env::Bus;
use crate::env::Processes;
use crate::stream::next_opt;
use crate::units::Stamp;

/// Everything the loop owns besides the machine.
struct Live {
    tx: watch::Sender<Lock>,
    machine: Locker,
    mode: LockerMode,
    epoch: Instant,
    logind: Logind,
    inhibitor: Option<OwnedFd>,
    /// What started the lock in place.
    cause: Option<LockCause>,
    /// The crash marker in this session's runtime directory; none without one.
    marker: Option<PathBuf>,
    processes: Arc<dyn Processes>,
}

/// Runs until `commands` closes. Inert (`LockerMode::Off`) until the first `Configure`/`Use`.
pub(super) async fn run(
    bus: Bus,
    runtime: Option<PathBuf>,
    processes: Arc<dyn Processes>,
    tx: watch::Sender<Lock>,
    mut commands: mpsc::UnboundedReceiver<LockerCommand>,
) {
    let mut live = Live {
        tx,
        machine: Locker::default(),
        mode: LockerMode::Off,
        epoch: Instant::now(),
        logind: logind::connect(&bus).await,
        inhibitor: None,
        cause: None,
        marker: runtime.as_deref().map(marker_path),
        processes,
    };
    loop {
        let deadline = next_deadline(&live.machine).map(|at| live.instant(at));
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command).await,
                None => break,
            },
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
            LockerCommand::Configure => {
                let processes = self.processes.clone();
                let mode = tokio::task::spawn_blocking(move || detect(processes.as_ref()))
                    .await
                    .unwrap_or(LockerMode::Off);
                self.use_mode(mode).await;
            }
            LockerCommand::Use(mode) => self.use_mode(mode).await,
            LockerCommand::Lock => {
                let now = self.now();
                self.feed(LockerEvent::Requested(Cause::Action, now)).await;
            }
            // sill's own idle turned the displays off.
            LockerCommand::IdleLock => {
                let now = self.now();
                self.feed(LockerEvent::Requested(Cause::Idle, now)).await;
            }
            LockerCommand::Shell(event) => {
                self.keep_marker(event);
                let event = match event {
                    ShellLockEvent::Locked => LockerEvent::Reported,
                    ShellLockEvent::Unlocked
                    | ShellLockEvent::Finished
                    | ShellLockEvent::Refused => LockerEvent::Exited,
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
        // A sill that died while its own lock held left the marker: lock again at once, so the
        // person gets a prompt on the session the compositor kept locked.
        let found = match &self.marker {
            Some(path) if path.exists() => LockMarker::Present,
            Some(_) | None => LockMarker::Absent,
        };
        if restore_on_start(&self.mode, found) == Restore::LockNow {
            let now = self.now();
            let event = LockerEvent::Requested(Cause::Action, now);
            self.feed_as(event, Some(LockCause::Restored)).await;
        }
    }

    /// Keep the crash marker while the own lock is held (module doc of [`super::marker`]).
    fn keep_marker(&self, event: ShellLockEvent) {
        let (LockerMode::Own { .. }, Some(path)) = (&self.mode, &self.marker) else {
            return;
        };
        match event {
            ShellLockEvent::Locked => {
                let written = path
                    .parent()
                    .map_or(Ok(()), std::fs::create_dir_all)
                    .and_then(|()| std::fs::write(path, b"locked\n"));
                if let Err(error) = written {
                    eprintln!("sill: writing the lock marker {}: {error}", path.display());
                }
            }
            ShellLockEvent::Unlocked | ShellLockEvent::Finished | ShellLockEvent::Refused => {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    /// Steps the machine with `event` and performs what it asks, feeding back any failure.
    async fn feed(&mut self, event: LockerEvent) {
        self.feed_as(event, cause_of(&event)).await;
    }

    /// [`Self::feed`], naming the cause a lock it starts gets.
    async fn feed_as(&mut self, event: LockerEvent, why: Option<LockCause>) {
        let was = self.machine.phase.public();
        let mut pending = vec![event];
        while let Some(event) = pending.pop() {
            let (next, effects) = step(self.machine, event);
            self.machine = next;
            for effect in effects {
                pending.extend(self.perform(effect).await);
            }
        }
        self.cause = cause_after(self.cause, was, self.machine.phase.public(), why);
        self.publish();
    }

    async fn perform(&mut self, effect: LockerEffect) -> Option<LockerEvent> {
        match effect {
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
        let lock = Lock {
            phase: self.machine.phase.public(),
            cause: self.cause,
        };
        self.tx.send_if_modified(|current| {
            let changed = *current != lock;
            *current = lock;
            changed
        });
    }
}

/// Resolves the mode against this session (blocking: `/proc`).
fn detect(processes: &dyn Processes) -> LockerMode {
    let manager = if processes.running("cosmic-session").is_empty() {
        SessionManager::Other
    } else {
        SessionManager::CosmicSession
    };
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    LockerMode::for_host(Host::of(manager, &desktop))
}
