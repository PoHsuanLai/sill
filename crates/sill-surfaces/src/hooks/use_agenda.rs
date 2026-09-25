//! `use_agenda`: today, and the calendar events around it (M10).

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Agenda;

use super::use_services;

/// Today, and the calendar events around it (M10).
pub fn use_agenda() -> ReadSignal<Agenda> {
    use_watch(use_services().agenda)
}
