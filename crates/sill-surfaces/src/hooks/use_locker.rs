//! `use_locker`: the lock screen's phase and who draws it.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Lock;

use super::use_services;

/// The lock screen's phase and who draws it.
pub fn use_locker() -> ReadSignal<Lock> {
    use_watch(use_services().lock)
}
