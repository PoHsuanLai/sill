//! `use_bluetooth`: BlueZ's adapter and devices.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Bluetooth;

use super::use_services;

/// BlueZ's adapter and devices.
pub fn use_bluetooth() -> ReadSignal<Bluetooth> {
    use_watch(use_services().bluetooth)
}
