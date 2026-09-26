//! The auth service's loop.
//!
//! Loads the [`super::Account`] once, at the first command (so a service nobody locks with never
//! reads anything: uid from `/proc/self`'s owner, [`super::user_name_in_passwd`] over
//! `/etc/passwd`, AccountsService's `IconFile` over the system bus when it answers within 1 s,
//! [`super::avatar_source`]); feeds every command and every finished check through
//! [`super::lockout::step`] with a monotonic [`crate::units::Stamp`]; runs each `Check` with
//! `authenticator.check` on `rt`'s blocking pool, tagged with a [`Round`] so a verdict that comes
//! back after a `Reset` is dropped; turns `Fake` into a verdict without the authenticator (after
//! [`FAKE_CHECK_MS`], so the surface sees `Checking` first, as for a real check; a fake accept
//! skips a lockout's wait: debug builds only); ticks at [`super::lockout::next_deadline`];
//! publishes [`super::Auth`] (a `LockedOut` wait as wall-clock `UnixMs`); ends when every control
//! is gone. The secret lives only in the `Submit` and the check's `Attempt`, both dropped (and
//! zeroed) when the check ends; it is never logged or published.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};

use super::authenticator::Authenticator;
use super::lockout::{Gate, Lockout, LockoutIn, LockoutOut, LockoutPolicy, next_deadline, step};
use super::types::{
    Account, Attempt, Auth, AuthCommand, AuthPhase, FakeVerdict, Outcome, RealName, Secret,
    UserName, Verdict,
};
use super::{avatar_source, user_name_in_passwd};
use crate::notifications::UnixMs;
use crate::units::Stamp;

/// How long a made-up verdict (`sill debug auth`) takes, so the surface sees `Checking` and the
/// field shakes again on every wrong one.
pub const FAKE_CHECK_MS: u64 = 250;

/// How long AccountsService gets to name the person's picture.
const ACCOUNTS_TIMEOUT: Duration = Duration::from_secs(1);

/// Which check a verdict answers: a `Reset` (or a fake taking over) starts a new round.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Round(u64);

/// A finished check, tagged.
struct Finished {
    round: Round,
    verdict: Verdict,
}

/// Whether the account has been read yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Loaded {
    Not,
    Yes,
}

/// Everything the loop owns.
struct Live {
    tx: watch::Sender<Auth>,
    lockout: Lockout,
    round: Round,
    epoch: Instant,
    loaded: Loaded,
    authenticator: Arc<dyn Authenticator>,
    rt: Handle,
    done: mpsc::UnboundedSender<Finished>,
}

pub(super) async fn run(
    tx: watch::Sender<Auth>,
    mut commands: mpsc::UnboundedReceiver<AuthCommand>,
    authenticator: Arc<dyn Authenticator>,
    rt: Handle,
) {
    let (done, mut finished) = mpsc::unbounded_channel();
    let mut live = Live {
        tx,
        lockout: Lockout::default(),
        round: Round::default(),
        epoch: Instant::now(),
        loaded: Loaded::Not,
        authenticator,
        rt,
        done,
    };
    loop {
        let deadline = next_deadline(&live.lockout).map(|at| live.instant(at));
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command).await,
                None => break,
            },
            Some(check) = finished.recv() => live.finished(check),
            () = sleep_until(deadline) => {
                let now = live.now();
                live.feed(LockoutIn::Tick(now), &Verdict::plain(Outcome::Refused));
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
        self.epoch + Duration::from_millis(stamp.0)
    }

    async fn command(&mut self, command: AuthCommand) {
        if self.loaded == Loaded::Not {
            self.loaded = Loaded::Yes;
            let account = load_account().await;
            self.tx.send_modify(|auth| auth.account = account);
        }
        match command {
            AuthCommand::Submit(secret) => self.submit(secret),
            AuthCommand::Reset => self.reset(),
            AuthCommand::Fake(verdict) => self.fake(verdict),
        }
    }

    fn submit(&mut self, secret: Secret) {
        let now = self.now();
        let (next, out) = step(
            self.lockout,
            LockoutIn::Submit(now),
            &LockoutPolicy::DEFAULT,
        );
        self.lockout = next;
        if !out.contains(&LockoutOut::Check) {
            return;
        }
        let attempt = Attempt {
            user: self.tx.borrow().account.user.clone(),
            secret,
        };
        let (round, authenticator, done) = (
            self.round,
            Arc::clone(&self.authenticator),
            self.done.clone(),
        );
        self.rt.spawn_blocking(move || {
            let verdict = authenticator.check(&attempt);
            drop(attempt);
            let _ = done.send(Finished { round, verdict });
        });
        self.publish_out(&out, &Verdict::plain(Outcome::Refused));
    }

    fn reset(&mut self) {
        self.round = Round(self.round.0 + 1);
        let (next, _) = step(self.lockout, LockoutIn::Reset, &LockoutPolicy::DEFAULT);
        self.lockout = next;
        let phase = match next.gate {
            Gate::Waiting { until } => AuthPhase::LockedOut {
                until: self.wall(until),
            },
            Gate::Open | Gate::Checking => AuthPhase::Idle,
        };
        self.tx.send_if_modified(|auth| {
            let changed = auth.phase != phase || !auth.notes.is_empty();
            auth.phase = phase;
            auth.notes.clear();
            changed
        });
    }

    /// A made-up verdict: whatever check runs is dropped, a wait is skipped for an accept, and
    /// the verdict arrives after [`FAKE_CHECK_MS`] like a real one.
    fn fake(&mut self, fake: FakeVerdict) {
        let now = self.now();
        let outcome = match fake {
            FakeVerdict::Accept => Outcome::Accepted,
            FakeVerdict::Refuse => Outcome::Refused,
        };
        if let (Gate::Waiting { until }, FakeVerdict::Accept) = (self.lockout.gate, fake) {
            let (next, _) = step(
                self.lockout,
                LockoutIn::Tick(until),
                &LockoutPolicy::DEFAULT,
            );
            self.lockout = next;
        }
        if self.lockout.gate == Gate::Checking {
            self.round = Round(self.round.0 + 1);
        } else {
            let (next, out) = step(
                self.lockout,
                LockoutIn::Submit(now),
                &LockoutPolicy::DEFAULT,
            );
            self.lockout = next;
            if !out.contains(&LockoutOut::Check) {
                return;
            }
            self.publish_out(&out, &Verdict::plain(Outcome::Refused));
        }
        let (round, done) = (self.round, self.done.clone());
        self.rt.spawn(async move {
            tokio::time::sleep(Duration::from_millis(FAKE_CHECK_MS)).await;
            let _ = done.send(Finished {
                round,
                verdict: Verdict::plain(outcome),
            });
        });
    }

    fn finished(&mut self, check: Finished) {
        if check.round != self.round {
            return;
        }
        let now = self.now();
        let input = LockoutIn::Verdict(check.verdict.outcome.clone(), now);
        self.feed(input, &check.verdict);
    }

    /// Step the machine with `input` and publish what it says (`verdict` for its notes and
    /// fault).
    fn feed(&mut self, input: LockoutIn, verdict: &Verdict) {
        let (next, out) = step(self.lockout, input, &LockoutPolicy::DEFAULT);
        self.lockout = next;
        self.publish_out(&out, verdict);
    }

    fn publish_out(&self, out: &[LockoutOut], verdict: &Verdict) {
        let Some(phase) = out
            .iter()
            .rev()
            .find_map(|said| self.phase_of(*said, verdict))
        else {
            return;
        };
        let failures = self.lockout.failures;
        let notes = match phase {
            AuthPhase::Checking => Vec::new(),
            _ => verdict.notes.clone(),
        };
        self.tx.send_if_modified(|auth| {
            let changed = auth.phase != phase || auth.failures != failures || auth.notes != notes;
            auth.phase = phase;
            auth.failures = failures;
            auth.notes = notes;
            changed
        });
    }

    /// The published phase one output means (`Busy` means nothing new).
    fn phase_of(&self, said: LockoutOut, verdict: &Verdict) -> Option<AuthPhase> {
        match said {
            LockoutOut::Check => Some(AuthPhase::Checking),
            LockoutOut::Busy => None,
            LockoutOut::Accepted => Some(AuthPhase::Accepted),
            LockoutOut::Wrong => Some(AuthPhase::Wrong),
            LockoutOut::LockedOut { until } => Some(AuthPhase::LockedOut {
                until: self.wall(until),
            }),
            LockoutOut::Unavailable => match verdict.outcome {
                Outcome::Unavailable(fault) => Some(AuthPhase::Unavailable(fault)),
                Outcome::Accepted | Outcome::Refused => Some(AuthPhase::Idle),
            },
            LockoutOut::Reopened => Some(AuthPhase::Idle),
        }
    }

    /// A monotonic stamp as the wall clock's.
    fn wall(&self, stamp: Stamp) -> UnixMs {
        let ahead = i64::try_from(stamp.since(self.now())).unwrap_or(i64::MAX);
        UnixMs(wall_now().0.saturating_add(ahead))
    }
}

fn wall_now() -> UnixMs {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    UnixMs(i64::try_from(ms).unwrap_or(i64::MAX))
}

/// This session's person: the owner of this process, from `/etc/passwd`, with their picture.
async fn load_account() -> Account {
    use std::os::unix::fs::MetadataExt;
    let Ok(uid) = std::fs::metadata("/proc/self").map(|meta| meta.uid()) else {
        return Account::default();
    };
    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    let (user, name) = user_name_in_passwd(&passwd, uid).unwrap_or_else(|| {
        let user = std::env::var("USER").unwrap_or_default();
        (UserName(user.clone()), RealName(user))
    });
    let icon = tokio::time::timeout(ACCOUNTS_TIMEOUT, accounts_icon(uid))
        .await
        .ok()
        .flatten();
    let home = dirs::home_dir().unwrap_or_default();
    Account {
        user,
        name,
        avatar: avatar_source(icon, &home, Path::is_file),
    }
}

/// AccountsService's `IconFile` for `uid`, when it has one.
async fn accounts_icon(uid: u32) -> Option<PathBuf> {
    const ACCOUNTS: &str = "org.freedesktop.Accounts";
    let bus = zbus::Connection::system().await.ok()?;
    let reply = bus
        .call_method(
            Some(ACCOUNTS),
            "/org/freedesktop/Accounts",
            Some(ACCOUNTS),
            "FindUserById",
            &i64::from(uid),
        )
        .await
        .ok()?;
    let path: zbus::zvariant::OwnedObjectPath = reply.body().deserialize().ok()?;
    let user = zbus::Proxy::new(&bus, ACCOUNTS, path, "org.freedesktop.Accounts.User")
        .await
        .ok()?;
    let icon: String = user.get_property("IconFile").await.ok()?;
    (!icon.is_empty()).then(|| PathBuf::from(icon))
}
