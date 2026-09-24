//! `use_wallpapers`: every output's prepared wallpaper images.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Wallpapers;

use super::use_services;

/// Every output's prepared wallpaper images.
pub fn use_wallpapers() -> ReadSignal<Wallpapers> {
    use_watch(use_services().wallpapers)
}
