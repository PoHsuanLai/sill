//! sill's own lock screen on the daemon's side (M11 lane a; FINDINGS "M11 freeze" F544, F551):
//! the follower that turns the locker's and the auth service's state into a wish to hold the
//! session lock or not, the host-thread half that takes and drops shell-host's session lock
//! through [`host::SessionLockHost`], the prompt's output, `sill lock status`, and `sill debug
//! auth`.
//!
//! The flow: `sill lock` (or logind's Lock in a sill session, or a lock marker left by a sill
//! that died locked) steps the locker service to `Locking` with `drawer: Shell`;
//! [`spawn_follower`] sees [`LockWish::Held`] and sends `ShellMsg::Lock`; [`apply`] resets the
//! auth service, points the [`LockLink`] at the main display and locks with a
//! [`host::LockFactory`] of `LockScreen`s; `HostEvent::Lock(Locked)` becomes
//! `LockerCommand::Shell(Locked)` ([`on_host_event`]). A right password makes the auth service
//! `Accepted`: every lock surface plays its fade, and the prompt's surface calls
//! `shell_host::use_lock().unlock()` when its fade settles (shell-host sends `unlock_and_destroy`
//! at the next flush). The wish turns `Released` at the same moment; the follower sends it only
//! after [`UNLOCK_BACKSTOP_MS`] and only if the lock is still held, so [`apply`]'s
//! `unlock_session` is a backstop for a surface that never finished its fade (its output gone).
//! `Unlocked` / `Finished` become `Shell(Unlocked)` / `Shell(Finished)` and `AuthCommand::Reset`;
//! a refusal is `Shell(Refused)`.
//!
//! Safety: nothing here runs in a test against a real session. `sill debug auth accept` is
//! refused unless the daemon was built with `debug`, and the acceptance scripts only ever run
//! such a daemon on the nested compositor (`wayland-1`), never on `wayland-0`.

pub mod host;

use shell_host::{Focus, HostEvent, HostHandle, LockId, SurfaceId};
use sill_ipc::{
    Failure, FailureKind, FakeAuth, LockReport, ReportDrawer, ReportLockPhase, Response,
};
use sill_services::{Auth, Lock, LockDrawer, LockPhase, Services};
use sill_surfaces::surfaces::lock::LockLink;
use tokio::runtime::Handle;

use crate::msg::ShellMsg;
use crate::outputs::Outputs;
use host::{LockFactory, SessionLockHost};

/// How long after `Accepted` the daemon unlocks if the lock surfaces have not (their fade is
/// `--t-move`, well under this).
pub const UNLOCK_BACKSTOP_MS: u64 = 1_500;

/// Whether the daemon should hold shell-host's session lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LockWish {
    Held,
    Released,
}

/// The wish from the two services: held while the locker draws with the shell and is not
/// `Unlocked`, unless the auth service has `Accepted` (the unlock).
pub fn lock_wish(lock: &Lock, auth: &Auth) -> LockWish {
    todo!("M11 fill: a-lock ({lock:?} {:?})", auth.phase)
}

/// The lock the host thread holds.
#[derive(Debug, Default)]
pub struct ShellLock {
    pub id: Option<LockId>,
}

/// Follow `services.lock` and `services.auth` on `rt` and send `ShellMsg::Lock(wish)` to the
/// host at start and whenever [`lock_wish`] changes (a `Released` after `Accepted` only once
/// [`UNLOCK_BACKSTOP_MS`] has passed with the wish unchanged), until the host is gone. Lane a
/// fills it; a no-op until then.
pub fn spawn_follower(rt: &Handle, services: &Services, host: HostHandle<ShellMsg>) {
    let _ = (rt, services, host);
}

/// On the host thread: make `held` match `wish`. Held and none held: `AuthCommand::Reset`,
/// the [`LockLink`] to the main display, `lock_session(factory())`, and on a refusal
/// `LockerCommand::Shell(Refused)` (an `InProgress` lock is adopted as held instead). Released
/// and one held: `unlock_session`. Anything else: nothing. Lane a fills it; a no-op until then.
pub fn apply(
    host: &mut impl SessionLockHost,
    held: &mut ShellLock,
    wish: LockWish,
    factory: impl FnOnce() -> LockFactory,
    link: &LockLink,
    outputs: &Outputs,
    services: &Services,
) {
    let _ = (host, held, wish, factory, link, outputs, services);
}

/// A host event about the lock ([`host::lock_event`]): `Locked`, `Unlocked`, `Finished` to the
/// locker service as `ShellLockEvent`s (and on `Unlocked` / `Finished` `AuthCommand::Reset`,
/// forgetting the id); a lock surface gaining the keyboard ([`host::SessionLockHost::
/// lock_surface_output`]) moves the [`LockLink`] there. Lane a fills it; a no-op until then.
pub fn on_host_event(
    host: &impl SessionLockHost,
    held: &mut ShellLock,
    event: &HostEvent<ShellMsg>,
    link: &LockLink,
    services: &Services,
) {
    let _ = (
        host,
        held,
        link,
        services,
        host::lock_event(event),
        lock_focus(event),
    );
}

/// A keyboard focus gained, for moving the prompt.
fn lock_focus(event: &HostEvent<ShellMsg>) -> Option<SurfaceId> {
    match event {
        HostEvent::KeyboardFocus {
            id,
            focus: Focus::Gained,
        } => Some(*id),
        _ => None,
    }
}

/// `sill lock status`.
pub fn status(services: &Services) -> Response {
    Response::Lock(report_of(&services.lock.borrow()))
}

/// The locker's state as the wire's.
pub fn report_of(lock: &Lock) -> LockReport {
    LockReport {
        phase: match lock.phase {
            LockPhase::Unlocked => ReportLockPhase::Unlocked,
            LockPhase::Locking => ReportLockPhase::Locking,
            LockPhase::Locked => ReportLockPhase::Locked,
        },
        drawer: match lock.drawer {
            LockDrawer::Program => ReportDrawer::Program,
            LockDrawer::Shell => ReportDrawer::Shell,
        },
    }
}

/// `sill debug auth accept|refuse`: a made-up verdict for the lock screen's check. Only a
/// daemon built with `debug` takes it; any other refuses, since an accepted verdict unlocks.
pub fn fake_auth(services: &Services, fake: FakeAuth) -> Response {
    fake_auth_in_this_build(services, fake)
}

#[cfg(feature = "debug")]
fn fake_auth_in_this_build(services: &Services, fake: FakeAuth) -> Response {
    let verdict = match fake {
        FakeAuth::Accept => sill_services::auth::FakeVerdict::Accept,
        FakeAuth::Refuse => sill_services::auth::FakeVerdict::Refuse,
    };
    match services
        .auth_control
        .send(sill_services::AuthCommand::Fake(verdict))
    {
        Ok(()) => Response::Done,
        Err(error) => Response::Failed(Failure {
            kind: FailureKind::Internal,
            message: format!("auth: {error}"),
        }),
    }
}

#[cfg(not(feature = "debug"))]
fn fake_auth_in_this_build(services: &Services, fake: FakeAuth) -> Response {
    let _ = services;
    Response::Failed(Failure {
        kind: FailureKind::Unsupported,
        message: format!("built without the debug feature (a made-up {fake:?} could unlock)"),
    })
}

#[cfg(test)]
mod tests;
