//! `use_wifi`: NetworkManager's wireless radio, access points and saved networks.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Wifi;

use super::use_services;

/// NetworkManager's wireless radio, access points and saved networks.
pub fn use_wifi() -> ReadSignal<Wifi> {
    use_watch(use_services().wifi)
}
