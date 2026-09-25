//! `use_displays`: every display's brightness, DDC/CI controls and backlight.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Displays;

use super::use_services;

/// The displays service's published state (the OSD's brightness level).
pub fn use_displays() -> ReadSignal<Displays> {
    use_watch(use_services().displays)
}
