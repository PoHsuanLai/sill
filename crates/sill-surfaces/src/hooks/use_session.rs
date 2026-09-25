//! `use_session`: What logind allows: the power menu's buttons.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Session;

use super::use_services;

/// What logind allows: the power menu's buttons.
pub fn use_session() -> ReadSignal<Session> {
    use_watch(use_services().session)
}
