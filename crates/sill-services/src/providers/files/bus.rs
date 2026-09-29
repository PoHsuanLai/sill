//! Async bus calls from the provider's synchronous `query`. The runner calls every provider on
//! the blocking pool (`spawn_blocking`), where `Handle::block_on` is the documented way back
//! into async code; with no runtime at all (an example, a plain test) a current-thread runtime is
//! made for the one call. Also whether a bus name is owned or only activatable, for
//! Tracker's detection and the launcher's reveal.

use std::future::Future;

use tokio::runtime::{Builder, Handle};
use zbus::Connection;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

/// Whether a well-known name is on the bus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum NameState {
    /// Someone holds it now.
    Owned,
    /// Nobody holds it, but a call to it would start its service.
    Activatable,
    Missing,
}

/// `future`'s output, or `None` when no runtime could run it.
pub(super) fn block_on<F: Future>(future: F) -> Option<F::Output> {
    match Handle::try_current() {
        Ok(handle) => Some(handle.block_on(future)),
        Err(_) => Builder::new_current_thread()
            .enable_all()
            .build()
            .ok()
            .map(|runtime| runtime.block_on(future)),
    }
}

/// `name`'s state on `connection`'s bus: `NameHasOwner`, then `ListActivatableNames`. Starts
/// nothing.
pub(crate) async fn name_state(connection: &Connection, name: &str) -> zbus::Result<NameState> {
    let dbus = DBusProxy::new(connection).await?;
    if dbus.name_has_owner(BusName::try_from(name)?).await? {
        return Ok(NameState::Owned);
    }
    let activatable = dbus.list_activatable_names().await?;
    Ok(if activatable.iter().any(|n| n.as_str() == name) {
        NameState::Activatable
    } else {
        NameState::Missing
    })
}
