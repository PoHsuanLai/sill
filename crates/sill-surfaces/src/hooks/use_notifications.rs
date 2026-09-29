//! `use_notifications`: the live banners and who holds the notifications bus name.

use dioxus::prelude::{ReadSignal, ReadableExt, use_memo};
use shell_host::use_watch;
use sill_services::Notifications;

use super::use_services;

/// The live banners, newest first, and the server's status: what the banner column draws.
pub fn use_notifications() -> ReadSignal<Notifications> {
    let notes = use_watch(use_services().notifications);
    ReadSignal::from(use_memo(move || notes.read().live.clone()))
}
