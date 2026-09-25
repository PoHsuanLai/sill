//! `use_shots`: the screenshot the floating thumbnail shows (M10).

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Shots;

use super::use_services;

/// The screenshot the floating thumbnail shows (M10).
pub fn use_shots() -> ReadSignal<Shots> {
    use_watch(use_services().shots)
}
