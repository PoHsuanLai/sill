//! `use_clock`: the bar clock's text, re-rendered once a minute.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::ClockText;

use super::use_services;

/// The bar clock's text, re-rendered once a minute.
pub fn use_clock() -> ReadSignal<ClockText> {
    use_watch(use_services().clock)
}
