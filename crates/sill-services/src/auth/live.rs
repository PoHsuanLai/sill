//! The auth service's loop.
//!
//! Loads the [`super::Account`] once, at the first command (so a service nobody locks with never
//! reads anything: uid from `/proc/self`'s owner, [`super::user_name_in_passwd`] over
//! `/etc/passwd`, AccountsService's `IconFile` over the system bus when it answers within 1 s,
//! [`super::avatar_source`]); feeds every command and every finished check through
//! [`super::gate::step`]; runs each `Check` with `authenticator.check` on `rt`'s blocking pool,
//! tagged with a [`Round`] so a verdict that comes back after a `Reset` is dropped; turns `Fake`
//! into a verdict without the authenticator (after [`FAKE_CHECK_MS`], so the surface sees
//! `Checking` first, as for a real check: debug builds only); publishes [`super::Auth`]; ends
//! when every control is gone. There is no lockout wait. The secret lives only in the `Submit` and
//! the check's `Attempt`, both dropped (and zeroed) when the check ends; it is never logged or
//! published.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};

use super::authenticator::Authenticator;
use super::gate::{Checks, Gate, GateIn, GateOut, step};
use super::types::{
    Account, Attempt, Auth, AuthCommand, AuthPhase, FakeVerdict, Outcome, RealName, Secret,
    UserName, Verdict,
};
use super::{AccountPlace, avatar_source, user_name_in_passwd};
use crate::env::Bus;

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
    checks: Checks,
    round: Round,
    loaded: Loaded,
    account: AccountPlace,
    authenticator: Arc<dyn Authenticator>,
    rt: Handle,
    done: mpsc::UnboundedSender<Finished>,
}

pub(super) async fn run(
    tx: watch::Sender<Auth>,
    mut commands: mpsc::UnboundedReceiver<AuthCommand>,
    account: AccountPlace,
    authenticator: Arc<dyn Authenticator>,
) {
    let rt = Handle::current();
    let (done, mut finished) = mpsc::unbounded_channel();
    let mut live = Live {
        tx,
        checks: Checks::default(),
        round: Round::default(),
        loaded: Loaded::Not,
        account,
        authenticator,
        rt,
        done,
    };
    loop {
        tokio::select! {
            command = commands.recv() => match command {
                Some(command) => live.command(command).await,
                None => break,
            },
            Some(check) = finished.recv() => live.finished(check),
        }
    }
}

impl Live {
    async fn command(&mut self, command: AuthCommand) {
        if self.loaded == Loaded::Not {
            self.loaded = Loaded::Yes;
            let account = load_account(&self.account).await;
            self.tx.send_modify(|auth| auth.account = account);
        }
        match command {
            AuthCommand::Submit(secret) => self.submit(secret),
            AuthCommand::Reset => self.reset(),
            AuthCommand::Fake(verdict) => self.fake(verdict),
            AuthCommand::Grace(_ticket) => self.grace(),
        }
    }

    fn submit(&mut self, secret: Secret) {
        let (next, out) = step(self.checks, GateIn::Submit);
        self.checks = next;
        if !out.contains(&GateOut::Check) {
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

    /// The lock grace: `Accepted` as a right password would publish it, with no check, so the lock
    /// surfaces unlock through their one path. Ignored while a check runs (the
    /// person is already typing; their own verdict stands).
    fn grace(&mut self) {
        match self.checks.gate {
            Gate::Checking => {}
            Gate::Open => {
                let (checking, _) = step(self.checks, GateIn::Submit);
                let (next, out) = step(checking, GateIn::Verdict(Outcome::Accepted));
                self.checks = next;
                self.publish_out(&out, &Verdict::plain(Outcome::Accepted));
            }
        }
    }

    fn reset(&mut self) {
        self.round = Round(self.round.0 + 1);
        let (next, _) = step(self.checks, GateIn::Reset);
        self.checks = next;
        let phase = AuthPhase::Idle;
        self.tx.send_if_modified(|auth| {
            let changed = auth.phase != phase || !auth.notes.is_empty();
            auth.phase = phase;
            auth.notes.clear();
            changed
        });
    }

    /// A made-up verdict: whatever check runs is dropped, and the verdict arrives after
    /// [`FAKE_CHECK_MS`] like a real one.
    fn fake(&mut self, fake: FakeVerdict) {
        let outcome = match fake {
            FakeVerdict::Accept => Outcome::Accepted,
            FakeVerdict::Refuse => Outcome::Refused,
        };
        match self.checks.gate {
            Gate::Checking => self.round = Round(self.round.0 + 1),
            Gate::Open => {
                let (next, out) = step(self.checks, GateIn::Submit);
                self.checks = next;
                if !out.contains(&GateOut::Check) {
                    return;
                }
                self.publish_out(&out, &Verdict::plain(Outcome::Refused));
            }
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
        let input = GateIn::Verdict(check.verdict.outcome.clone());
        let (next, out) = step(self.checks, input);
        self.checks = next;
        self.publish_out(&out, &check.verdict);
    }

    /// Publish what the machine said (`verdict` for its notes and fault).
    fn publish_out(&self, out: &[GateOut], verdict: &Verdict) {
        let Some(phase) = out.iter().rev().find_map(|said| phase_of(*said, verdict)) else {
            return;
        };
        let failures = self.checks.failures;
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
}

/// The published phase one output means (`Busy` means nothing new).
fn phase_of(said: GateOut, verdict: &Verdict) -> Option<AuthPhase> {
    match said {
        GateOut::Check => Some(AuthPhase::Checking),
        GateOut::Busy => None,
        GateOut::Accepted => Some(AuthPhase::Accepted),
        GateOut::Wrong => Some(AuthPhase::Wrong),
        GateOut::Unavailable => match verdict.outcome {
            Outcome::Unavailable(fault) => Some(AuthPhase::Unavailable(fault)),
            Outcome::Accepted | Outcome::Refused => Some(AuthPhase::Idle),
        },
    }
}

/// This session's person: the owner of this process, from `/etc/passwd`, with their picture.
async fn load_account(place: &AccountPlace) -> Account {
    use std::os::unix::fs::MetadataExt;
    let Ok(uid) = std::fs::metadata("/proc/self").map(|meta| meta.uid()) else {
        return Account::default();
    };
    let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
    let (user, name) = user_name_in_passwd(&passwd, uid).unwrap_or_else(|| {
        let user = std::env::var("USER").unwrap_or_default();
        (UserName(user.clone()), RealName(user))
    });
    let icon = tokio::time::timeout(ACCOUNTS_TIMEOUT, accounts_icon(&place.system, uid))
        .await
        .ok()
        .flatten();
    let home = &place.home;
    Account {
        user,
        name,
        avatar: avatar_source(icon, home, Path::is_file),
    }
}

/// AccountsService's `IconFile` for `uid`, when it has one.
async fn accounts_icon(system: &Bus, uid: u32) -> Option<PathBuf> {
    const ACCOUNTS: &str = "org.freedesktop.Accounts";
    let bus = system.connect().await.ok()??;
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
