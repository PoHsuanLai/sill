//! A daemon that starts after sill, or restarts under it: the two halves services share to follow
//! a well-known bus name. A service runs its session (seed, then follow) only while the name has
//! an owner, and starts it over when the owner changes:
//!
//! ```text
//! loop {
//!     owner::appeared(&connection, NAME).await;
//!     tokio::select! {
//!         done = session(..) => break,                    // the commands closed
//!         () = owner::changed(&connection, NAME) => gone(&tx),   // left, or replaced
//!     }
//! }
//! ```
//!
//! Waiting on `NameOwnerChanged` instead of retrying on a timer costs nothing while the daemon is
//! absent, and a daemon that never comes leaves the service at its default value.

use zbus::Connection;
use zbus::fdo::{DBusProxy, NameOwnerChangedStream};
use zbus::names::{BusName, OwnedUniqueName};

use crate::stream::next;

/// The unique name that owns `name` right now, when anyone does and the bus answers.
pub(crate) async fn current(connection: &Connection, name: &str) -> Option<OwnedUniqueName> {
    let dbus = DBusProxy::new(connection).await.ok()?;
    dbus.get_name_owner(BusName::try_from(name).ok()?)
        .await
        .ok()
}

/// A `NameOwnerChanged` stream for `name` alone, and whether it has an owner right now (the
/// stream is opened first, so no change falls between the two).
async fn watch(connection: &Connection, name: &str) -> Option<(NameOwnerChangedStream, bool)> {
    let dbus = DBusProxy::new(connection).await.ok()?;
    let changes = dbus
        .receive_name_owner_changed_with_args(&[(0, name)])
        .await
        .ok()?;
    let owned = dbus
        .name_has_owner(zbus::names::BusName::try_from(name).ok()?)
        .await
        .ok()?;
    Some((changes, owned))
}

/// Resolves once `name` has an owner (at once when it already does). Never resolves when the bus
/// cannot be asked, which leaves the service as it is.
pub(crate) async fn appeared(connection: &Connection, name: &str) {
    let Some((mut changes, owned)) = watch(connection, name).await else {
        return std::future::pending().await;
    };
    if owned {
        return;
    }
    while let Some(change) = next(&mut changes).await {
        if change.args().is_ok_and(|args| args.new_owner().is_some()) {
            return;
        }
    }
    std::future::pending().await
}

/// Resolves when the owner of `name` changes: it left, or another took over (state read from
/// the old one is stale either way). Never resolves when the bus cannot be asked.
pub(crate) async fn changed(connection: &Connection, name: &str) {
    let Some((mut changes, owned)) = watch(connection, name).await else {
        return std::future::pending().await;
    };
    if !owned {
        return;
    }
    if next(&mut changes).await.is_none() {
        std::future::pending::<()>().await;
    }
}

/// How a session ended without the owner changing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ended {
    /// The commands closed: the service is done.
    Commands,
    /// The daemon answered badly (a read failed): wait for its owner to change, then try again.
    Unreadable,
}
