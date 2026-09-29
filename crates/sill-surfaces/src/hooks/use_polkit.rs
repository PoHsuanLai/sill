//! `use_polkit`: sill's polkit agent: the request the prompt shows.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Polkit;

use super::use_services;

/// sill's polkit agent: the request the prompt shows.
pub fn use_polkit() -> ReadSignal<Polkit> {
    use_watch(use_services().polkit)
}
