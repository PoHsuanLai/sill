//! `use_pins`: the dock's pinned apps (`dock.json`).

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::DockPins;

use super::use_services;

/// The dock's pinned apps (`dock.json`).
pub fn use_pins() -> ReadSignal<DockPins> {
    use_watch(use_services().pins)
}
