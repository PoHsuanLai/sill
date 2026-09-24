//! `use_launches`: the app scopes the launch service started.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Launches;

use super::use_services;

/// The app scopes the launch service started.
pub fn use_launches() -> ReadSignal<Launches> {
    use_watch(use_services().launches)
}
