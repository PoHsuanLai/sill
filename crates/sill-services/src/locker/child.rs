//! Starting a locker and hearing from it: its ready signal ([`Readiness`]) and its exit, both
//! sent back to the live loop tagged with the start's [`Generation`] so a late word from an
//! older locker is not taken for the current one's.
//!
//! The child is never killed by sill, not even when sill exits: unlocking is the locker's
//! business, and a locker killed while it holds the lock leaves the compositor locked with
//! nothing to unlock it.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc::UnboundedSender;

use super::choice::{LockerPlan, Readiness};
use crate::programs::{Pid, still_running};

/// Which start a word belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(super) struct Generation(pub u64);

/// What a locker said.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Heard {
    Ready,
    Exited,
}

/// A word from one start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Word {
    pub generation: Generation,
    pub heard: Heard,
}

/// How often an adopted locker (not our child, so it cannot be waited on) is checked.
const ADOPTED_POLL: Duration = Duration::from_millis(500);

/// How long the journal is followed for a ready line: past the machine's give-up, after which
/// the locker is taken as locked anyway (and `journalctl` is stopped with the future).
const JOURNAL_BOUND: Duration = Duration::from_millis(super::machine::READY_GIVE_UP_MS);

/// Starts `plan`; its pid, or `None` when it could not be started (the loop then hears
/// `Exited`).
pub(super) fn start(
    plan: &LockerPlan,
    generation: Generation,
    words: UnboundedSender<Word>,
) -> Option<Pid> {
    let stdout = match plan.readiness {
        Readiness::StdoutLine => Stdio::piped(),
        Readiness::Journal(_) | Readiness::Grace(_) => Stdio::null(),
    };
    let spawned = Command::new(&plan.program)
        .args(&plan.args)
        .envs(plan.env.iter().map(|(k, v)| (k, v)))
        .stdin(Stdio::null())
        .stdout(stdout)
        .kill_on_drop(false)
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(error) => {
            eprintln!("sill: locker: starting {}: {error}", plan.program.display());
            let _ = words.send(Word {
                generation,
                heard: Heard::Exited,
            });
            return None;
        }
    };
    let pid = child.id().map(Pid);
    listen(&plan.readiness, &mut child, pid, generation, words.clone());
    tokio::spawn(async move {
        let _ = child.wait().await;
        let _ = words.send(Word {
            generation,
            heard: Heard::Exited,
        });
    });
    pid
}

/// Watches a locker sill did not start (found running) until it is gone.
pub(super) fn watch_adopted(pid: Pid, generation: Generation, words: UnboundedSender<Word>) {
    tokio::spawn(async move {
        while still_running(pid).is_some() {
            tokio::time::sleep(ADOPTED_POLL).await;
        }
        let _ = words.send(Word {
            generation,
            heard: Heard::Exited,
        });
    });
}

fn listen(
    readiness: &Readiness,
    child: &mut Child,
    pid: Option<Pid>,
    generation: Generation,
    words: UnboundedSender<Word>,
) {
    let ready = move || {
        let _ = words.send(Word {
            generation,
            heard: Heard::Ready,
        });
    };
    match readiness {
        Readiness::StdoutLine => {
            if let Some(stdout) = child.stdout.take() {
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stdout).lines();
                    if let Ok(Some(_)) = lines.next_line().await {
                        ready();
                    }
                    // Keep draining so the locker never blocks on a full pipe.
                    while let Ok(Some(_)) = lines.next_line().await {}
                });
            }
        }
        Readiness::Grace(ms) => {
            let grace = Duration::from_millis(*ms);
            tokio::spawn(async move {
                tokio::time::sleep(grace).await;
                ready();
            });
        }
        Readiness::Journal(text) => {
            if let Some(pid) = pid {
                let text = text.clone();
                tokio::spawn(async move {
                    let heard = tokio::time::timeout(JOURNAL_BOUND, journal_says(pid, &text));
                    if let Ok(Some(())) = heard.await {
                        ready();
                    }
                });
            }
        }
    }
}

/// Follows the journal for `pid` until a line contains `text`; `None` if journalctl is missing
/// or ends first (the machine's give-up then takes the locker as locked).
async fn journal_says(pid: Pid, text: &str) -> Option<()> {
    let mut follow = Command::new("journalctl")
        .args(["--follow", "--output=cat", "--lines=all"])
        .arg(format!("_PID={}", pid.0))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .ok()?;
    let mut lines = BufReader::new(follow.stdout.take()?).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if line.contains(text) {
            return Some(());
        }
    }
    None
}
