//! `use_toplevels`: COSMIC's toplevels, most recently activated first.

use dioxus::prelude::{ReadSignal, ReadableExt, use_memo};
use shell_host::use_watch;
use sill_services::Toplevels;

use super::use_services;

/// COSMIC's toplevels, most recently activated first.
pub fn use_toplevels() -> ReadSignal<Toplevels> {
    let wl = use_watch(use_services().cosmic_wl);
    ReadSignal::from(use_memo(move || wl.read().toplevels.clone()))
}
