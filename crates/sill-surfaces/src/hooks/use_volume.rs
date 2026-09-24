//! `use_volume`: the default sink's volume and mute.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::Volume;

use super::use_services;

/// The default sink's volume and mute.
pub fn use_volume() -> ReadSignal<Volume> {
    use_watch(use_services().volume)
}
