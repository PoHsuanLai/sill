//! The power profile state's pure transition.

use super::{PowerProfiles, ProfileEvent};
use crate::units::Presence;

/// `profiles` after `event`. `Found` marks the service present; `Gone` returns to the default.
pub fn step(profiles: PowerProfiles, event: ProfileEvent) -> PowerProfiles {
    match event {
        ProfileEvent::Found => PowerProfiles {
            presence: Presence::Present,
            ..profiles
        },
        ProfileEvent::Active(active) => PowerProfiles { active, ..profiles },
        ProfileEvent::Offered(offered) => PowerProfiles {
            offered,
            ..profiles
        },
        ProfileEvent::Degraded(degraded) => PowerProfiles {
            degraded,
            ..profiles
        },
        ProfileEvent::Gone => PowerProfiles::default(),
    }
}
