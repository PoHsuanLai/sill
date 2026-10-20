//! A daemon that starts after sill, or restarts under it: [`follow_owner`] runs a service's
//! session (seed, then follow) only while a well-known bus name has an owner, and starts it over
//! when the owner changes. [`appeared`] and [`changed`] are its two halves, for the few services
//! whose loop is not that shape.
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

/// Runs `session` while `name` has an owner and `gone` each time the owner left or was replaced,
/// until a session says [`Ended::Commands`]. `state` is what both closures work on (the commands
/// receiver, or the service's own struct), handed to each in turn so they never borrow it at
/// once. A daemon that never comes leaves the service waiting, at no cost.
pub(crate) async fn follow_owner<S>(
    connection: &Connection,
    name: &str,
    state: &mut S,
    mut session: impl AsyncFnMut(&mut S) -> Ended,
    mut gone: impl FnMut(&mut S),
) {
    loop {
        appeared(connection, name).await;
        let ended = tokio::select! {
            ended = session(&mut *state) => Some(ended),
            () = changed(connection, name) => None,
        };
        match ended {
            Some(Ended::Commands) => return,
            Some(Ended::Unreadable) => changed(connection, name).await,
            None => {}
        }
        gone(&mut *state);
    }
}

/// Runs `task` while `name` has an owner, once per owner, and `gone` each time the owner left or
/// was replaced; returns when a task finishes (its receiver is gone, so there is nothing left to
/// tell). For a follower that holds no state of its own between owners.
pub(crate) async fn while_owned(
    connection: &Connection,
    name: &str,
    mut task: impl AsyncFnMut(),
    mut gone: impl FnMut(),
) {
    follow_owner(
        connection,
        name,
        &mut (),
        async |_| {
            task().await;
            Ended::Commands
        },
        |_| gone(),
    )
    .await;
}

/// How a session ended without the owner changing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ended {
    /// The commands closed: the service is done.
    Commands,
    /// The daemon answered badly (a read failed): wait for its owner to change, then try again.
    Unreadable,
}
