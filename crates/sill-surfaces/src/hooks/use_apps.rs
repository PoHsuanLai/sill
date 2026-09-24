//! `use_apps`: the application index.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::AppIndex;

use super::use_services;

/// The application index.
pub fn use_apps() -> ReadSignal<AppIndex> {
    use_watch(use_services().apps)
}
