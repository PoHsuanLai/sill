//! `use_launcher_entry`: LauncherEntry badges.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Badges;

use super::use_services;

/// Every app's LauncherEntry count, progress and urgency.
pub fn use_launcher_entry() -> ReadSignal<Badges> {
    use_watch(use_services().badges)
}
