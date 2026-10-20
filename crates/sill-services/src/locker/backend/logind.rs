//! The locker's half of logind: this session's `Lock` signal, the manager's `PrepareForSleep`,
//! the sleep delay inhibitor, and `LockSession` for a resident locker.

use logind_zbus::manager::{InhibitType, ManagerProxy, PrepareForSleepStream};
use logind_zbus::session::{LockStream, SessionProxy};
use zbus::zvariant::OwnedFd;

use crate::env::{Bus, ConnectError};
use crate::login_session::session_path;

/// Whether logind is, or will be, there.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Standing {
    /// No system bus by choice: nothing to wait for.
    #[default]
    Absent,
    /// The bus or logind was not reachable: try again later.
    Waiting,
    /// Connected.
    Up,
}

/// What the live loop listens to and calls. Every part is optional: without a system bus sill
/// still locks on its own Lock action.
#[derive(Default)]
pub(crate) struct Logind {
    pub standing: Standing,
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

impl Logind {
    fn waiting() -> Logind {
        Logind {
            standing: Standing::Waiting,
            ..Logind::default()
        }
    }
}

/// Connects and subscribes; whatever fails is left out, and a bus or logind that is not there
/// yet is [`Standing::Waiting`] so the loop asks again.
pub(crate) async fn connect(bus: &Bus, session_id: Option<&str>) -> Logind {
    let connection = match bus.try_connect().await {
        Ok(connection) => connection,
        Err(ConnectError::NoBus(_)) => return Logind::default(),
        Err(ConnectError::Refused(_)) => return Logind::waiting(),
    };
    let Ok(manager) = ManagerProxy::new(&connection).await else {
        return Logind::waiting();
    };
    let sleep = manager.receive_prepare_for_sleep().await.ok();
    let session = match session_path(&connection, &manager, session_id).await {
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
        standing: Standing::Up,
        manager: Some(manager),
        session_id,
        sleep,
        lock,
    }
}

/// `Inhibit("sleep", "sill", "lock before sleep", "delay")`.
pub(crate) async fn inhibit(manager: &ManagerProxy<'_>) -> Option<OwnedFd> {
    manager
        .inhibit(InhibitType::Sleep, "sill", "lock before sleep", "delay")
        .await
        .ok()
}
