//! The locker's half of logind: this session's `Lock` signal, the manager's `PrepareForSleep`,
//! the sleep delay inhibitor, and `LockSession` for a resident locker.

use logind_zbus::manager::{InhibitType, ManagerProxy, PrepareForSleepStream};
use logind_zbus::session::{LockStream, SessionProxy};
use logind_zbus::user::UserProxy;
use zbus::Connection;
use zbus::zvariant::{OwnedFd, OwnedObjectPath};

use crate::env::Bus;

/// What the live loop listens to and calls. Every part is optional: without a system bus sill
/// still locks on its own Lock action.
#[derive(Default)]
pub(crate) struct Logind {
    pub manager: Option<ManagerProxy<'static>>,
    pub session_id: Option<String>,
    pub sleep: Option<PrepareForSleepStream>,
    pub lock: Option<LockStream>,
}

impl std::fmt::Debug for Logind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Logind")
            .field("session_id", &self.session_id)
            .finish_non_exhaustive()
    }
}

/// Connects and subscribes; whatever fails is left out.
pub(crate) async fn connect(bus: &Bus) -> Logind {
    let Ok(Some(connection)) = bus.connect().await else {
        return Logind::default();
    };
    let Ok(manager) = ManagerProxy::new(&connection).await else {
        return Logind::default();
    };
    let sleep = manager.receive_prepare_for_sleep().await.ok();
    let session = match session_path(&connection, &manager).await {
        Some(path) => SessionProxy::builder(&connection)
            .path(path)
            .ok()
            .map(|b| b.build()),
        None => None,
    };
    let session = match session {
        Some(build) => build.await.ok(),
        None => None,
    };
    let (session_id, lock) = match &session {
        Some(session) => (session.id().await.ok(), session.receive_lock().await.ok()),
        None => (None, None),
    };
    Logind {
        manager: Some(manager),
        session_id,
        sleep,
        lock,
    }
}

/// This process's session: by pid (sill started from the session script), else
/// `XDG_SESSION_ID`, else the user's display session (sill as a systemd user service, which
/// logind places in no session).
pub(crate) async fn session_path(
    connection: &Connection,
    manager: &ManagerProxy<'_>,
) -> Option<OwnedObjectPath> {
    if let Ok(path) = manager.get_session_by_PID(std::process::id()).await {
        return Some(path);
    }
    if let Ok(id) = std::env::var("XDG_SESSION_ID")
        && let Ok(path) = manager.get_session(&id).await
    {
        return Some(path);
    }
    let uid = uid()?;
    let user_path = manager.get_user(uid).await.ok()?;
    let user = UserProxy::builder(connection)
        .path(user_path)
        .ok()?
        .build()
        .await
        .ok()?;
    let display = user.display().await.ok()?;
    manager.get_session(display.id()).await.ok()
}

fn uid() -> Option<u32> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata("/proc/self").ok().map(|m| m.uid())
}

/// `Inhibit("sleep", "sill", "lock before sleep", "delay")`.
pub(crate) async fn inhibit(manager: &ManagerProxy<'_>) -> Option<OwnedFd> {
    manager
        .inhibit(InhibitType::Sleep, "sill", "lock before sleep", "delay")
        .await
        .ok()
}
