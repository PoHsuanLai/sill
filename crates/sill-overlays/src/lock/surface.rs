//! The lock screen as a [`LockSurface`]: one document per output, made by shell-host's session lock.

use super::layer::config;
use super::view::lock;
use shell_host::HostRoot;
use sill_ui_kit::links::Links;
use sill_ui_kit::surface::LockSurface;

/// sill's own lock screen: the clock on every output, the password prompt on one.
pub const LOCK: LockSurface = LockSurface {
    name: "lock",
    config,
    needs,
    root: lock,
};

/// The links the lock's document reads by type, given to each lock root: only `LockLink`, which
/// says which output shows the password prompt (`body.rs`).
fn needs(root: HostRoot, links: &Links) -> HostRoot {
    root.context(links.lock.clone())
}
