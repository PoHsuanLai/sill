//! `use_idle`: sill's own idle service (Q420 B): its mode, phase and the dim it publishes.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Idle;

use super::use_services;

/// sill's own idle service: its mode, phase and the dim it publishes (FINDINGS F902).
pub fn use_idle() -> ReadSignal<Idle> {
    use_watch(use_services().idle)
}
