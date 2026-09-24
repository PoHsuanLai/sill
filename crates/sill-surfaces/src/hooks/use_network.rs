//! `use_network`: NetworkManager's primary connection.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Network;

use super::use_services;

/// NetworkManager's primary connection.
pub fn use_network() -> ReadSignal<Network> {
    use_watch(use_services().network)
}
