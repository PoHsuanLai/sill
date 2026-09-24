//! `use_results`: the launcher's ranked results for the newest query.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Results;

use super::use_services;

/// The launcher's ranked results for the newest query.
pub fn use_results() -> ReadSignal<Results> {
    use_watch(use_services().results)
}
