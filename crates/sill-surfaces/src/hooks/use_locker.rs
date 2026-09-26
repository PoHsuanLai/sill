//! `use_locker`: the lock screen's phase and who draws it (M7, M11).

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Lock;

use super::use_services;

/// The lock screen's phase and who draws it (M7, M11).
pub fn use_locker() -> ReadSignal<Lock> {
    use_watch(use_services().lock)
}
