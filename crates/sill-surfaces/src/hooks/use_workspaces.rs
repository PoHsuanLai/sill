//! `use_workspaces`: COSMIC's workspace groups (the `cosmic_wl` thread).

use dioxus::prelude::{ReadSignal, ReadableExt, use_memo};
use shell_host::use_watch;
use sill_services::Workspaces;

use super::use_services;

/// COSMIC's workspace groups (the `cosmic_wl` thread).
pub fn use_workspaces() -> ReadSignal<Workspaces> {
    let wl = use_watch(use_services().cosmic_wl);
    ReadSignal::from(use_memo(move || wl.read().workspaces.clone()))
}
