//! logind over the system bus: probes `CanSuspend`, `CanReboot` and `CanPowerOff` (and this
//! session's object for Lock and Log Out), publishes them through [`sill_model::session::step`], and
//! carries out [`SessionCommand`]s.

use std::sync::Arc;

use logind_zbus::manager::{IsSupported, ManagerProxy};
use logind_zbus::session::SessionProxy;
use sill_model::session::{Offer, Session, SessionAction, SessionCommand, SessionEvent, step};
use sill_model::units::Presence;
use tokio::sync::{mpsc, watch};
use zbus::Connection;

use crate::control::Control;
use crate::env::{ActionReach, Bus, SessionActions};
use crate::locker::LockerCommand;
use crate::stream::next_opt;

/// Runs until `commands` closes. Without a bus or logind, `tx` holds what `actions` offers
/// without it (`Session::default()` for logind's own). Every action goes through `actions`
/// first, so a dry run never reaches logind or the locker.
pub(crate) async fn run(
    bus: Bus,
    actions: Arc<dyn SessionActions>,
    tx: watch::Sender<Session>,
    mut commands: mpsc::UnboundedReceiver<SessionCommand>,
    locker: Control<LockerCommand>,
) {
    let logind = connect(&bus).await;
    let logind = logind.as_ref();
    let actions = actions.as_ref();
    let mut session_id = refresh(logind, &tx, actions).await;
    let mut sleeping = match logind {
        Some(logind) => logind.manager.receive_prepare_for_sleep().await.ok(),
        None => None,
    };

    loop {
        tokio::select! {
            Some(signal) = next_opt(&mut sleeping) => {
                if let Ok(args) = signal.args()
                    && !*args.start()
                {
                    // Resuming: what logind allows can have changed (a lid-close policy, a
                    // fresh inhibitor) since it was last asked.
                    session_id = refresh(logind, &tx, actions).await;
                }
            }
            command = commands.recv() => match command {
                Some(SessionCommand::Refresh) => {
                    session_id = refresh(logind, &tx, actions).await;
                }
                Some(SessionCommand::Run(SessionAction::Lock)) => {
                    // The locker service decides (its own locker, or logind's LockSession for
                    // COSMIC's resident one); never logind here, so sill nested in another
                    // desktop's session cannot lock that desktop.
                    if actions.act(SessionAction::Lock) == ActionReach::Logind {
                        let _gone_is_inert = locker.send(LockerCommand::Lock);
                    }
                }
                Some(SessionCommand::Run(action)) => {
                    let offer = tx.borrow().offer(action);
                    // An action not offered is dropped (matching what the launcher and the
                    // power menu show), same as a `Set` the daemon does not offer in the power
                    // profile service.
                    if offer != Offer::Unavailable
                        && actions.act(action) == ActionReach::Logind
                        && let Some(logind) = logind
                    {
                        run_action(&logind.manager, session_id.as_deref(), action).await;
                    }
                }
                None => break,
            }
        }
    }
}

/// logind's manager on its bus.
struct Logind {
    connection: Connection,
    manager: ManagerProxy<'static>,
}

/// logind on `bus`, when the bus is there.
async fn connect(bus: &Bus) -> Option<Logind> {
    let connection = bus.connect().await.ok().flatten()?;
    let manager = ManagerProxy::new(&connection).await.ok()?;
    Some(Logind {
        connection,
        manager,
    })
}

/// Publish what `actions` offers given logind's answers now (none without logind), and return
/// this process's session id when logind knows it.
async fn refresh(
    logind: Option<&Logind>,
    tx: &watch::Sender<Session>,
    actions: &dyn SessionActions,
) -> Option<String> {
    let (event, session_id) = match logind {
        Some(logind) => probe(&logind.connection, &logind.manager).await,
        None => (SessionEvent::Gone, None),
    };
    let next = actions.offers(step(*tx.borrow(), event));
    tx.send_if_modified(|shown| {
        let changed = *shown != next;
        *shown = next;
        changed
    });
    session_id
}

/// Re-reads every `Can*` and this process's session: the event it makes, and the session id
/// [`run_action`] needs for `LogOut`. `Gone` (and `None`) only when logind
/// answered none of the three `Can*` calls at all; one session object this process cannot find
/// (common outside a real login session, e.g. under a build sandbox) is a legitimate `Session`
/// with `lock`/`log_out` left `Unavailable`, not an unreachable daemon.
async fn probe(
    connection: &Connection,
    manager: &ManagerProxy<'_>,
) -> (SessionEvent, Option<String>) {
    let suspend = manager.can_suspend().await;
    let restart = manager.can_reboot().await;
    let power_off = manager.can_power_off().await;
    if suspend.is_err() && restart.is_err() && power_off.is_err() {
        return (SessionEvent::Gone, None);
    }

    let session_id = current_session_id(connection, manager).await;
    let lock_log_out = if session_id.is_some() {
        Offer::Available
    } else {
        Offer::Unavailable
    };
    let probed = Session {
        presence: Presence::Present,
        lock: lock_log_out,
        log_out: lock_log_out,
        restart: offer_of(restart),
        power_off: offer_of(power_off),
        suspend: offer_of(suspend),
    };
    (SessionEvent::Probed(probed), session_id)
}

/// This process's own session id (`Session.Id`, e.g. `"3"`), by way of `GetSessionByPID` and the
/// session object it names; `None` if logind cannot place this process in any session.
async fn current_session_id(connection: &Connection, manager: &ManagerProxy<'_>) -> Option<String> {
    let path = manager.get_session_by_PID(std::process::id()).await.ok()?;
    let session = SessionProxy::builder(connection)
        .path(path)
        .ok()?
        .build()
        .await
        .ok()?;
    session.id().await.ok()
}

/// Carries out `action` through logind.
async fn run_action(manager: &ManagerProxy<'_>, session_id: Option<&str>, action: SessionAction) {
    let _ = match action {
        // Routed to the locker service before this point.
        SessionAction::Lock => return,
        SessionAction::LogOut => {
            let Some(id) = session_id else { return };
            manager.terminate_session(id).await
        }
        SessionAction::Suspend => manager.suspend(true).await,
        SessionAction::Restart => manager.reboot(true).await,
        SessionAction::PowerOff => manager.power_off(true).await,
    };
}

/// logind's `Can*` result to an [`Offer`], by way of the frozen [`Offer::from_logind`] and
/// `IsSupported`'s own word (`enum_impl_str_conv`); an error is `Unavailable`, same as `no`.
fn offer_of(result: zbus::Result<IsSupported>) -> Offer {
    match result {
        Ok(supported) => Offer::from_logind(<&str>::from(supported)),
        Err(_) => Offer::Unavailable,
    }
}
