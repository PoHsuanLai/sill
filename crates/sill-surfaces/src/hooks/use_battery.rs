//! `use_battery`: UPower's display device.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Battery;

use super::use_services;

/// UPower's display device.
pub fn use_battery() -> ReadSignal<Battery> {
    use_watch(use_services().battery)
}
