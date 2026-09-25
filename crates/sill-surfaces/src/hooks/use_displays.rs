//! `use_displays`: The displays: outputs, DDC monitors, backlights (the brightness slider).

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Displays;

use super::use_services;

/// The displays: outputs, DDC monitors, backlights (the brightness slider).
pub fn use_displays() -> ReadSignal<Displays> {
    use_watch(use_services().displays)
}
