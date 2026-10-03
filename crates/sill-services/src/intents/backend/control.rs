//! sill as the control center of intentd: the terminal's standing grants and the way back from the
//! breaker. The connection owns `org.quire.Shell`, which the shipped `intentd.toml` gives the
//! `control` role, so these members answer it and nobody else.

use docket_core::ActionRef;
use docket_dbus::{ControlProxy, INTENTS_BUS};
use prov::{SpaceId, SpaceScope};
use tokio::sync::mpsc::UnboundedSender;
use zbus::Connection;

use crate::env::{NameState, name_state};
use crate::stream::next;

/// The Space a terminal's calls are made in (intentd gives every role without a session of its own
/// the implicit one of the `desktop` Space): the scope of "Resume the terminal".
const TERMINAL_SPACE: &str = "desktop";

/// The proxy to intentd's `Control`, when intentd is on the bus: a call to a name nobody owns
/// would ask the bus to start it, and opening the control center must not start a daemon.
async fn running(connection: &Connection) -> Result<ControlProxy<'static>, String> {
    match name_state(connection, INTENTS_BUS).await.map_err(text)? {
        NameState::Owned => ControlProxy::new(connection).await.map_err(text),
        NameState::Activatable | NameState::Missing => Err("intentd is not running".to_owned()),
    }
}

/// The standing grants, in intentd's order.
pub(super) async fn grants(connection: &Connection) -> Result<Vec<ActionRef>, String> {
    let proxy = running(connection).await?;
    let json = proxy.terminal_grants().await.map_err(text)?;
    serde_json::from_str(&json).map_err(text)
}

/// Withdraw one grant.
pub(super) async fn revoke(connection: &Connection, grant: &ActionRef) -> Result<(), String> {
    let proxy = running(connection).await?;
    let json = serde_json::to_string(grant).map_err(text)?;
    proxy.revoke_terminal_grant(&json).await.map_err(text)
}

/// Let the terminal's paused sessions take calls again.
pub(super) async fn resume(connection: &Connection) -> Result<(), String> {
    let proxy = running(connection).await?;
    let space = SpaceId::parse(TERMINAL_SPACE).map_err(text)?;
    let json = serde_json::to_string(&SpaceScope::Only(space)).map_err(text)?;
    proxy.resume(&json).await.map_err(text)
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

fn text(error: impl std::fmt::Display) -> String {
    error.to_string()
}
