use super::{PowerProfiles, Profile, ProfileDegraded, ProfileEvent, step};
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
        ..PowerProfiles::default()
    };
    let cases: Vec<(&str, PowerProfiles, ProfileEvent, PowerProfiles)> = vec![
        (
            "found",
            PowerProfiles::default(),
            ProfileEvent::Found,
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
        assert_eq!(step(before, event), want, "{name}");
    }
}
