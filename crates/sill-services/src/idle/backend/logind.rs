//! Idle's half of logind, on [`Reach::system`](crate::idle::Reach) (a private bus with a stand-in
//! in
//! every test): `PrepareForSleep`, `BlockInhibited` (polled: logind does not signal it),
//! `Suspend(false)` (never `interactive`) and this session's `SetIdleHint`. Whatever fails is
//! left out and logged once; idle still dims and turns the displays off without it.

use logind_zbus::manager::{ManagerProxy, PrepareForSleepStream};
use logind_zbus::session::SessionProxy;
use sill_model::idle::{IdleHint, LogindIdle, logind_idle};
use zbus::Connection;
use zbus::proxy::CacheProperties;

use crate::env::Bus;
use crate::login_session::session_path;

/// The connected half.
pub(crate) struct IdleLogind {
    manager: ManagerProxy<'static>,
    session: Option<SessionProxy<'static>>,
    pub(crate) sleep: Option<PrepareForSleepStream>,
}

impl std::fmt::Debug for IdleLogind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdleLogind").finish_non_exhaustive()
    }
}

/// A connection to `bus`; `None` when it is absent or cannot be reached.
pub(crate) async fn bus(bus: &Bus) -> Option<Connection> {
    match bus.connect().await {
        Ok(connection) => connection,
        Err(error) => {
            eprintln!("sill: idle: no bus at {:?}: {error}", bus.route());
            None
        }
    }
}

/// Connect to logind on `system`.
pub(crate) async fn connect(system: &Bus, session_id: Option<&str>) -> Option<IdleLogind> {
    let connection = bus(system).await?;
    let manager = match ManagerProxy::builder(&connection)
        .cache_properties(CacheProperties::No)
        .build()
        .await
    {
        Ok(manager) => manager,
        Err(error) => {
            eprintln!("sill: idle: no logind: {error}");
            return None;
        }
    };
    let sleep = manager.receive_prepare_for_sleep().await.ok();
    let session = match session_path(&connection, &manager, session_id).await {
        Some(path) => match SessionProxy::builder(&connection).path(path) {
            Ok(builder) => builder.build().await.ok(),
            Err(_) => None,
        },
        None => None,
    };
    if session.is_none() {
        eprintln!("sill: idle: no logind session; the idle hint is not set");
    }
    match manager.idle_action().await.as_deref() {
        Ok("ignore") | Err(_) => {}
        Ok(action) => eprintln!(
            "sill: idle: logind's IdleAction is {action:?}: sill's idle hint will trigger it too (F912)"
        ),
    }
    Some(IdleLogind {
        manager,
        session,
        sleep,
    })
}

impl IdleLogind {
    /// `BlockInhibited` read as [`LogindIdle`]; `Free` when it cannot be read.
    pub(crate) async fn blocked(&self) -> LogindIdle {
        match self.manager.block_inhibited().await {
            Ok(text) => logind_idle(&text),
            Err(_) => LogindIdle::Free,
        }
    }

    /// `Suspend(false)`; a failure (a `block` sleep inhibitor, polkit) is logged.
    pub(crate) async fn suspend(&self) {
        if let Err(error) = self.manager.suspend(false).await {
            eprintln!("sill: idle: logind refused to suspend: {error}");
        }
    }

    pub(crate) async fn idle_hint(&self, hint: IdleHint) {
        let Some(session) = &self.session else {
            return;
        };
        let idle = match hint {
            IdleHint::Idle => true,
            IdleHint::Active => false,
        };
        if let Err(error) = session.set_idle_hint(idle).await {
            eprintln!("sill: idle: SetIdleHint({idle}): {error}");
        }
    }
}
