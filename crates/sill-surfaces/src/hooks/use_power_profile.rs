//! `use_power_profile`: power-profiles-daemon's active and offered profiles.

use dioxus::prelude::ReadSignal;
use shell_host::use_watch;
use sill_services::PowerProfiles;

use super::use_services;

/// power-profiles-daemon's active and offered profiles.
pub fn use_power_profile() -> ReadSignal<PowerProfiles> {
    use_watch(use_services().power_profiles)
}
