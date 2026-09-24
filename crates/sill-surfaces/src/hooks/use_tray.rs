//! `use_tray`: every status notifier item, with its menu.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Tray;

use super::use_services;

/// Every status notifier item, with its menu.
pub fn use_tray() -> ReadSignal<Tray> {
    use_watch(use_services().tray)
}
