use keycap_protocol::{Mode, Status};

use crate::keycap::{KeycapState, UnitState, unit_state, watching_unit};

#[test]
fn the_managers_words_become_unit_states() {
    let cases: &[(&str, Option<UnitState>)] = &[
        ("active", Some(UnitState::Active)),
        ("reloading", Some(UnitState::Active)),
        ("activating", Some(UnitState::Activating)),
        ("deactivating", Some(UnitState::Activating)),
        ("failed", Some(UnitState::Failed)),
        ("maintenance", Some(UnitState::Failed)),
        ("inactive", Some(UnitState::Inactive)),
        ("", None),
        ("Active", None),
        ("something-new", None),
    ];
    for &(word, want) in cases {
        assert_eq!(unit_state(word), want, "{word:?}");
    }
}

#[test]
fn the_unit_is_asked_about_only_while_the_socket_is_down() {
    let connected = KeycapState::Connected {
        status: Status {
            mode: Mode::Running,
            keyboards: Vec::new(),
        },
    };
    let cases = [
        ("unknown", KeycapState::Unknown, true),
        ("restarting", KeycapState::Restarting, true),
        ("failed", KeycapState::Failed, true),
        ("connected", connected, false),
        ("not installed", KeycapState::NotInstalled, false),
    ];
    for (case, state, want) in cases {
        assert_eq!(watching_unit(&state), want, "{case}");
    }
}
