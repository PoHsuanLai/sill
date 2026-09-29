//! The power profile: a quire `SegmentedControl<Profile>` over `PowerProfiles::offered`
//! (labels from [`crate::surfaces::control_center::profile_label`]), set with
//! `ProfileCommand::Set`; `ProfileDegraded::Because` shown as a `SectionHeader` value. Absent
//! when power-profiles-daemon is. Reads `use_service::<PowerProfileService>`.

use dioxus::prelude::*;
use ds::{HeaderKind, SectionHeader, SegSize, SegmentedControl};
use sill_model::units::Presence;
use sill_services::{PowerProfileService, Profile, ProfileCommand, ProfileDegraded};

use crate::hooks::{use_control, use_service};
use crate::surfaces::control_center::profile_label;

/// The power profile control.
#[component]
pub fn PowerProfileControl() -> Element {
    let profiles = use_service::<PowerProfileService>()();
    let control = use_control::<PowerProfileService>();
    if profiles.presence == Presence::Absent {
        return rsx! {};
    }
    let options: Vec<(Profile, String)> = profiles
        .offered
        .iter()
        .map(|&profile| (profile, profile_label(profile).to_owned()))
        .collect();
    let degraded = match profiles.degraded {
        ProfileDegraded::No => None,
        ProfileDegraded::Because(reason) => Some(reason),
    };
    rsx! {
        div { class: "sill-cc-profile",
            SectionHeader { kind: HeaderKind::Field, text: "Power Mode".to_owned(), value: degraded }
            SegmentedControl::<Profile> {
                label: "Power Mode".to_owned(),
                options,
                value: profiles.active,
                size: SegSize::Small,
                onchange: move |profile: Profile| {
                    // A stopped service drops it: the control shows what it last published.
                    let _gone = control.send(ProfileCommand::Set(profile));
                },
            }
        }
    }
}
