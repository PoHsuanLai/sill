//! `use_auth`: the own lock screen's password check (M11).

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Auth;

use super::use_services;

/// The own lock screen's password check (M11).
pub fn use_auth() -> ReadSignal<Auth> {
    use_watch(use_services().auth)
}
