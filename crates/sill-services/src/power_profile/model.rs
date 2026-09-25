//! The power profile's state, the daemon's profile words, and the one reducer.

use serde::{Deserialize, Serialize};

use crate::units::Presence;

/// What power-profiles-daemon says, as the control center draws it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PowerProfiles {
    /// Whether the daemon answered at all.
    pub presence: Presence,
    /// Which bus name it answered on.
    pub bus: ProfileBus,
    /// `ActiveProfile`.
    pub active: Profile,
    /// `Profiles`, in the daemon's order: a machine without a performance driver offers two.
    pub offered: Vec<Profile>,
    /// `PerformanceDegraded`: why Performance is running below itself, if it is.
    pub degraded: ProfileDegraded,
}

/// One power profile, in the order the control offers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    PowerSaver,
    #[default]
    Balanced,
    Performance,
}

impl Profile {
    /// Every profile, left to right in the control.
    pub const ALL: [Profile; 3] = [Profile::PowerSaver, Profile::Balanced, Profile::Performance];

    /// The daemon's word for a profile (`power-saver`, `balanced`, `performance`); `None` for any
    /// other word. Compared whole.
    pub fn from_daemon(word: &str) -> Option<Profile> {
        match word {
            "power-saver" => Some(Profile::PowerSaver),
            "balanced" => Some(Profile::Balanced),
            "performance" => Some(Profile::Performance),
            _ => None,
        }
    }

    /// The word the daemon takes for this profile.
    pub fn daemon_word(self) -> &'static str {
        match self {
            Profile::PowerSaver => "power-saver",
            Profile::Balanced => "balanced",
            Profile::Performance => "performance",
        }
    }
}

/// Which name the daemon answered on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileBus {
    /// `org.freedesktop.UPower.PowerProfiles` (0.20 on).
    #[default]
    UPower,
    /// `net.hadess.PowerProfiles`.
    Hadess,
}

/// `PerformanceDegraded`: empty when Performance runs fully, else the daemon's reason word
/// (`lap-detected`, `high-operating-temperature`), shown as the control's detail.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ProfileDegraded {
    #[default]
    No,
    Because(String),
}

/// One change the daemon reported.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ProfileEvent {
    /// The daemon answered on `bus`.
    Found(ProfileBus),
    Active(Profile),
    Offered(Vec<Profile>),
    Degraded(ProfileDegraded),
    /// The daemon left the bus.
    Gone,
}

/// `profiles` after `event`. `Found` marks the service present; `Gone` returns to the default.
pub fn reduce_power_profiles(profiles: PowerProfiles, event: ProfileEvent) -> PowerProfiles {
    match event {
        ProfileEvent::Found(bus) => PowerProfiles {
            presence: Presence::Present,
            bus,
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

#[cfg(test)]
mod tests {
    use super::{
        PowerProfiles, Profile, ProfileBus, ProfileDegraded, ProfileEvent, reduce_power_profiles,
    };
    use crate::units::Presence;

    #[test]
    fn the_daemons_words_round_trip() {
        const CASES: &[(&str, Option<Profile>)] = &[
            ("power-saver", Some(Profile::PowerSaver)),
            ("balanced", Some(Profile::Balanced)),
            ("performance", Some(Profile::Performance)),
            ("Performance", None),
            ("performance-plus", None),
            ("", None),
        ];
        for &(word, want) in CASES {
            assert_eq!(Profile::from_daemon(word), want, "{word:?}");
        }
        for profile in Profile::ALL {
            assert_eq!(
                Profile::from_daemon(profile.daemon_word()),
                Some(profile),
                "{profile:?}"
            );
        }
    }

    #[test]
    fn events_reduce_to_the_expected_profiles() {
        let present = PowerProfiles {
            presence: Presence::Present,
            bus: ProfileBus::Hadess,
            ..PowerProfiles::default()
        };
        let cases: Vec<(&str, PowerProfiles, ProfileEvent, PowerProfiles)> = vec![
            (
                "found on the old name",
                PowerProfiles::default(),
                ProfileEvent::Found(ProfileBus::Hadess),
                present.clone(),
            ),
            (
                "the active profile moves",
                present.clone(),
                ProfileEvent::Active(Profile::Performance),
                PowerProfiles {
                    active: Profile::Performance,
                    ..present.clone()
                },
            ),
            (
                "two profiles offered",
                present.clone(),
                ProfileEvent::Offered(vec![Profile::PowerSaver, Profile::Balanced]),
                PowerProfiles {
                    offered: vec![Profile::PowerSaver, Profile::Balanced],
                    ..present.clone()
                },
            ),
            (
                "degraded",
                present.clone(),
                ProfileEvent::Degraded(ProfileDegraded::Because("lap-detected".to_owned())),
                PowerProfiles {
                    degraded: ProfileDegraded::Because("lap-detected".to_owned()),
                    ..present.clone()
                },
            ),
            (
                "the daemon leaves",
                present,
                ProfileEvent::Gone,
                PowerProfiles::default(),
            ),
        ];
        for (name, before, event, want) in cases {
            assert_eq!(reduce_power_profiles(before, event), want, "{name}");
        }
    }
}
