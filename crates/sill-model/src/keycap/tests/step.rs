use keycap_protocol::{Grab, Keyboard, KeyboardInfo, Mode, Status};

use crate::keycap::{Attention, KeycapEvent, KeycapState, UnitState, attention, step};

fn keyboard(name: &str, grab: Grab) -> Keyboard {
    Keyboard {
        info: KeyboardInfo {
            name: name.to_owned(),
            path: format!("/dev/input/{name}"),
            vendor: 0x05ac,
            product: 0x0250,
        },
        grab,
    }
}

fn status(mode: Mode, keyboards: &[(&str, Grab)]) -> Status {
    Status {
        mode,
        keyboards: keyboards.iter().map(|&(n, g)| keyboard(n, g)).collect(),
    }
}

fn running() -> Status {
    status(Mode::Running, &[("k1", Grab::Active)])
}

fn connected() -> KeycapState {
    KeycapState::Connected { status: running() }
}

#[test]
fn the_state_follows_the_socket_and_the_unit() {
    use KeycapEvent::{Disconnected, Hello, Status as Update, UnitState as Unit};
    use KeycapState::{Failed, NotInstalled, Restarting, Unknown};
    let paused = status(Mode::Paused, &[]);
    let cases: Vec<(&str, KeycapState, KeycapEvent, KeycapState)> = vec![
        ("hello connects", Unknown, Hello(running()), connected()),
        (
            "hello after a restart",
            Restarting,
            Hello(running()),
            connected(),
        ),
        (
            "hello after a failure",
            Failed,
            Hello(running()),
            connected(),
        ),
        (
            "a status replaces the last",
            connected(),
            Update(paused.clone()),
            KeycapState::Connected { status: paused },
        ),
        (
            "a drop while connected restarts",
            connected(),
            Disconnected,
            Restarting,
        ),
        (
            "a drop while unknown stays unknown",
            Unknown,
            Disconnected,
            Unknown,
        ),
        (
            "a drop after failure stays failed",
            Failed,
            Disconnected,
            Failed,
        ),
        (
            "a drop while absent stays absent",
            NotInstalled,
            Disconnected,
            NotInstalled,
        ),
        (
            "a drop while restarting stays",
            Restarting,
            Disconnected,
            Restarting,
        ),
        ("failed unit", Restarting, Unit(UnitState::Failed), Failed),
        (
            "failed unit while connected",
            connected(),
            Unit(UnitState::Failed),
            Failed,
        ),
        ("no unit", Unknown, Unit(UnitState::NotFound), NotInstalled),
        (
            "activating",
            Failed,
            Unit(UnitState::Activating),
            Restarting,
        ),
        (
            "activating while connected",
            connected(),
            Unit(UnitState::Activating),
            Restarting,
        ),
        (
            "stopped on purpose",
            connected(),
            Unit(UnitState::Inactive),
            Unknown,
        ),
        (
            "active keeps a live connection",
            connected(),
            Unit(UnitState::Active),
            connected(),
        ),
        (
            "active while unknown waits for hello",
            Unknown,
            Unit(UnitState::Active),
            Unknown,
        ),
        (
            "active after failure is recovering",
            Failed,
            Unit(UnitState::Active),
            Restarting,
        ),
        (
            "active after install is starting",
            NotInstalled,
            Unit(UnitState::Active),
            Restarting,
        ),
    ];
    for (name, from, event, want) in cases {
        assert_eq!(step(from, event), want, "{name}");
    }
}

#[test]
fn a_crash_that_recovers_is_never_attention() {
    let events = [
        KeycapEvent::Disconnected,
        KeycapEvent::UnitState(UnitState::Activating),
        KeycapEvent::UnitState(UnitState::Active),
        KeycapEvent::Hello(running()),
    ];
    let mut state = connected();
    for event in events {
        state = step(state, event);
        if !matches!(state, KeycapState::Connected { .. }) {
            assert_eq!(attention(&state), None, "{state:?}");
        }
    }
    assert_eq!(state, connected());
}

#[test]
fn attention_names_what_the_person_must_know() {
    let conflict = |name: &str| {
        Some(Attention::Conflict {
            device: name.to_owned(),
        })
    };
    let cases: Vec<(&str, KeycapState, Option<Attention>)> = vec![
        ("unknown", KeycapState::Unknown, None),
        ("restarting", KeycapState::Restarting, None),
        ("not installed", KeycapState::NotInstalled, None),
        ("failed", KeycapState::Failed, Some(Attention::Failed)),
        ("running and grabbed", connected(), None),
        (
            "no keyboards attached",
            KeycapState::Connected {
                status: status(Mode::Running, &[]),
            },
            None,
        ),
        (
            "waiting for a key release is not a problem",
            KeycapState::Connected {
                status: status(Mode::Running, &[("k1", Grab::WaitingForRelease)]),
            },
            None,
        ),
        (
            "one keyboard conflicts",
            KeycapState::Connected {
                status: status(
                    Mode::Running,
                    &[("k1", Grab::Active), ("k2", Grab::Conflict)],
                ),
            },
            conflict("k2"),
        ),
        (
            "paused",
            KeycapState::Connected {
                status: status(Mode::Paused, &[("k1", Grab::Active)]),
            },
            Some(Attention::Paused),
        ),
        (
            "a conflict outranks a pause",
            KeycapState::Connected {
                status: status(Mode::Paused, &[("k1", Grab::Conflict)]),
            },
            conflict("k1"),
        ),
    ];
    for (name, state, want) in cases {
        assert_eq!(attention(&state), want, "{name}");
    }
}
