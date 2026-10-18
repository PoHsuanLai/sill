//! The lock screen as a [`LockSurface`]: one document per output, made by shell-host's session lock.

use super::layer::config;
use super::view::lock;
use sill_ui_kit::surface::LockSurface;

/// sill's own lock screen: the clock on every output, the password prompt on one.
pub const LOCK: LockSurface = LockSurface {
    name: "lock",
    config,
    root: lock,
};
