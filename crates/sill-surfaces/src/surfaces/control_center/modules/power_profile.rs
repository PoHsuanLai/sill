//! The power profile: a quire `SegmentedControl<Profile>` over `PowerProfiles::offered`
//! (labels from [`crate::surfaces::control_center::profile_label`]), set with
//! `ProfileCommand::Set`; `ProfileDegraded::Because` shown as a `SectionHeader` value. Absent
//! when power-profiles-daemon is. Reads `use_power_profile`.

use dioxus::prelude::*;

/// The power profile control.
#[component]
pub fn PowerProfileControl() -> Element {
    todo!("M5 fill: control-center (c)")
}
