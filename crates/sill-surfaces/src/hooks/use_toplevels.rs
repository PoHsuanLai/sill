//! `use_toplevels`: COSMIC's toplevels, most recently activated first.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Toplevels;

use super::use_services;

/// COSMIC's toplevels, most recently activated first.
pub fn use_toplevels() -> ReadSignal<Toplevels> {
    use_watch(use_services().toplevels)
}
