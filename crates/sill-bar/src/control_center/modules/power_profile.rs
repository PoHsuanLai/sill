//! The power profile: a quire `SegmentedControl<Profile>` over `PowerProfiles::offered`
//! (labels from [`crate::control_center::profile_label`]), set with
//! `ProfileCommand::Set`; `ProfileDegraded::Because` shown as a `SectionHeader` value. Absent
//! when power-profiles-daemon is. Reads `use_service::<PowerProfileService>`.

use dioxus::prelude::*;
use ds::{Choice, ControlSize, Tracking};
use ds::{HeaderKind, SectionHeader, SegmentedControl};
use sill_model::power_profile::{Profile, ProfileCommand, ProfileDegraded};
use sill_model::units::Presence;
use sill_services::PowerProfileService;

use crate::control_center::profile_label;
use sill_ui_kit::hooks::{use_control, use_service};

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
                choices: Choice::pairs(options),
                tracking: Tracking::SelectOne(profiles.active),
                size: ControlSize::Mini,
                onchange: move |profile: Profile| {
                    // A stopped service drops it: the control shows what it last published.
                    let _gone = control.send(ProfileCommand::Set(profile));
                },
            }
        }
    }
}
