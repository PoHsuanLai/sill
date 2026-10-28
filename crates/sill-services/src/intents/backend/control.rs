//! sill as the control center of intentd: the terminal's standing grants and the way back from the
//! breaker. The connection owns `org.quire.Shell`, which the shipped `intentd.toml` gives the
//! `control` role, so these members answer it and nobody else.

use docket_dbus::{ControlProxy, INTENTS_BUS};
use docket_permissions_core::StandingGrantId;
use docket_vocab::ActionRef;
use prov::{SpaceId, SpaceScope};
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;

use crate::call_error::CallError;
use crate::env::{NameState, name_state};
use crate::stream::next;

/// The Space a terminal's calls are made in (intentd gives every role without a session of its own
/// the implicit one of the `desktop` Space): the scope of "Resume the terminal".
const TERMINAL_SPACE: &str = "desktop";

/// The proxy to intentd's `Control`, when intentd is on the bus: a call to a name nobody owns
/// would ask the bus to start it, and opening the control center must not start a daemon.
async fn running(connection: &Connection) -> Result<ControlProxy<'static>, CallError> {
    match name_state(connection, INTENTS_BUS).await? {
        NameState::Owned => ControlProxy::new(connection).await.map_err(CallError::from),
        NameState::Activatable | NameState::Missing => {
            Err(CallError::Absent("intentd is not running"))
        }
    }
}

/// The standing grants, in intentd's order.
pub(super) async fn grants(connection: &Connection) -> Result<Vec<ActionRef>, CallError> {
    let proxy = running(connection).await?;
    let json = proxy.terminal_grants().await?;
    serde_json::from_str(&json).map_err(CallError::from)
}

/// Withdraw one grant.
pub(super) async fn revoke(connection: &Connection, grant: &ActionRef) -> Result<(), CallError> {
    let proxy = running(connection).await?;
    let json = serde_json::to_string(grant)?;
    proxy
        .revoke_terminal_grant(&json)
        .await
        .map_err(CallError::from)
}

/// Take away the "always ask" entry `id` names (a sheet's Undo): the next call it covered runs
/// by the ordinary rules again.
pub(super) async fn undo_limit(
    connection: &Connection,
    id: &StandingGrantId,
) -> Result<(), CallError> {
    let proxy = running(connection).await?;
    let json = serde_json::to_string(id)?;
    proxy
        .revoke_standing_grant(&json)
        .await
        .map_err(CallError::from)
}

/// Let the terminal's paused sessions take calls again.
pub(super) async fn resume(connection: &Connection) -> Result<(), CallError> {
    let proxy = running(connection).await?;
    let space = SpaceId::parse(TERMINAL_SPACE).map_err(CallError::invalid)?;
    let json = serde_json::to_string(&SpaceScope::Only(space))?;
    proxy.resume(&json).await.map_err(CallError::from)
}

/// What the control announces that the terminal's state follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Signalled {
    /// The breaker paused a session.
    Tripped,
    /// A scope was resumed.
    Resumed,
}

/// Forward the control's signals to `to` until the bus or the receiver goes. The signals come
/// from the owner of intentd's name whenever it has one, so a restart of intentd is followed.
pub(super) async fn forward(connection: &Connection, to: UnboundedSender<Signalled>) {
    let Ok(proxy) = ControlProxy::new(connection).await else {
        return;
    };
    let (Ok(mut tripped), Ok(mut resumed)) = (
        proxy.receive_breaker_tripped().await,
        proxy.receive_resumed().await,
    ) else {
        return;
    };
    loop {
        let heard = tokio::select! {
            Some(_) = next(&mut tripped) => Signalled::Tripped,
            Some(_) = next(&mut resumed) => Signalled::Resumed,
            else => return,
        };
        if to.send(heard).is_err() {
            return;
        }
    }
}
