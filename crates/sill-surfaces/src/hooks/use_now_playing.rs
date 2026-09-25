//! `use_now_playing`: Every MPRIS player; `Media::current` is the one to show.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Media;

use super::use_services;

/// Every MPRIS player; `Media::current` is the one to show.
pub fn use_now_playing() -> ReadSignal<Media> {
    use_watch(use_services().media)
}
