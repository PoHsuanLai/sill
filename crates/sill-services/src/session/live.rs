//! logind over the system bus: probes `CanSuspend`, `CanReboot` and `CanPowerOff` (and this
//! session's object for Lock and Log Out), publishes them through [`super::reduce_session`], and
//! carries out [`SessionCommand`]s.

use std::pin::Pin;

use logind_zbus::manager::{IsSupported, ManagerProxy};
use logind_zbus::session::SessionProxy;
use tokio::sync::{mpsc, watch};
use zbus::Connection;
use zbus::export::futures_core::Stream;

use super::SessionCommand;
use super::model::{Offer, Session, SessionAction, SessionEvent, reduce_session};
use crate::control::Control;
use crate::locker::LockerCommand;
use crate::units::Presence;

/// Runs until `commands` closes. Without a bus or logind, `tx` stays at `Session::default()`.
pub(super) async fn run(
    tx: watch::Sender<Session>,
    mut commands: mpsc::UnboundedReceiver<SessionCommand>,
    locker: Control<LockerCommand>,
) {
    let Ok(connection) = Connection::system().await else {
        return;
    };
    let Ok(manager) = ManagerProxy::new(&connection).await else {
        return;
    };

    let mut session_id = probe(&connection, &tx, &manager).await;
    let mut sleeping = manager.receive_prepare_for_sleep().await.ok();

    loop {
        tokio::select! {
            Some(signal) = next_opt(&mut sleeping) => {
                if let Ok(args) = signal.args()
                    && !*args.start()
                {
                    // Resuming: what logind allows can have changed (a lid-close policy, a
                    // fresh inhibitor) since it was last asked.
                    session_id = probe(&connection, &tx, &manager).await;
                }
            }
            command = commands.recv() => match command {
                Some(SessionCommand::Refresh) => {
                    session_id = probe(&connection, &tx, &manager).await;
                }
                Some(SessionCommand::Run(SessionAction::Lock)) => {
                    // The locker service decides (its own locker, or logind's LockSession for
                    // COSMIC's resident one); never logind here, so sill nested in another
                    // desktop's session cannot lock that desktop.
                    let _gone_is_inert = locker.send(LockerCommand::Lock);
                }
                Some(SessionCommand::Run(action)) => {
                    let offer = tx.borrow().offer(action);
                    run_action(&manager, session_id.as_deref(), offer, action).await;
                }
                None => break,
            }
        }
    }
}

/// Re-reads every `Can*` and this process's session, publishes the result, and returns the
/// session id [`run_action`] needs for `Lock` and `LogOut`. `Gone` (and `None`) only when logind
/// answered none of the three `Can*` calls at all; one session object this process cannot find
/// (common outside a real login session, e.g. under a build sandbox) is a legitimate `Session`
/// with `lock`/`log_out` left `Unavailable`, not an unreachable daemon.
async fn probe(
    connection: &Connection,
    tx: &watch::Sender<Session>,
    manager: &ManagerProxy<'_>,
) -> Option<String> {
    let suspend = manager.can_suspend().await;
    let restart = manager.can_reboot().await;
    let power_off = manager.can_power_off().await;
    if suspend.is_err() && restart.is_err() && power_off.is_err() {
        apply(tx, SessionEvent::Gone);
        return None;
    }

    let session_id = current_session_id(connection, manager).await;
    let lock_log_out = if session_id.is_some() {
        Offer::Available
    } else {
        Offer::Unavailable
    };
    apply(
        tx,
        SessionEvent::Probed(Session {
            presence: Presence::Present,
            lock: lock_log_out,
            log_out: lock_log_out,
            restart: offer_of(restart),
            power_off: offer_of(power_off),
            suspend: offer_of(suspend),
        }),
    );
    session_id
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

/// Carries out `action` if `offer` allows it (matching what the launcher and the power menu
/// would show); an action logind does not offer is silently dropped, same as a `Set` the daemon
/// does not offer in the power profile service.
async fn run_action(
    manager: &ManagerProxy<'_>,
    session_id: Option<&str>,
    offer: Offer,
    action: SessionAction,
) {
    if offer == Offer::Unavailable {
        return;
    }
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

/// Reduces `event` onto the receiver's current value and publishes the result.
fn apply(tx: &watch::Sender<Session>, event: SessionEvent) {
    let next = reduce_session(*tx.borrow(), event);
    let _ = tx.send(next);
}

/// Polls one `Unpin` stream for its next item, for use as a `tokio::select!` branch.
async fn next<S>(stream: &mut S) -> Option<S::Item>
where
    S: Stream + Unpin,
{
    std::future::poll_fn(|cx| Pin::new(&mut *stream).poll_next(cx)).await
}

/// [`next`] for a stream that might not exist: `None` never resolves, so the `tokio::select!`
/// branch it backs simply never fires instead of needing its own guard.
async fn next_opt<S>(stream: &mut Option<S>) -> Option<S::Item>
where
    S: Stream + Unpin,
{
    match stream {
        Some(stream) => next(stream).await,
        None => std::future::pending().await,
    }
}
