use sill_launcher::AppId;

use super::{DockPins, PinAt, PinEvent, step};

fn pins(ids: &[&str]) -> DockPins {
    DockPins {
        pinned: ids.iter().map(|s| AppId((*s).to_owned())).collect(),
    }
}

fn id(s: &str) -> AppId {
    AppId(s.to_owned())
}

#[test]
fn events_reduce_to_the_expected_pins() {
    let cases: Vec<(&str, DockPins, PinEvent, DockPins)> = vec![
        (
            "pin at the end",
            pins(&["a", "b"]),
            PinEvent::Pin {
                app: id("c"),
                at: PinAt::End,
            },
            pins(&["a", "b", "c"]),
        ),
        (
            "pin at a slot",
            pins(&["a", "b"]),
            PinEvent::Pin {
                app: id("c"),
                at: PinAt::Index(1),
            },
            pins(&["a", "c", "b"]),
        ),
        (
            "pin past the end clamps",
            pins(&["a"]),
            PinEvent::Pin {
                app: id("c"),
                at: PinAt::Index(9),
            },
            pins(&["a", "c"]),
        ),
        (
            "pinning a pinned app moves it",
            pins(&["a", "b", "c"]),
            PinEvent::Pin {
                app: id("a"),
                at: PinAt::End,
            },
            pins(&["b", "c", "a"]),
        ),
        (
            "unpin",
            pins(&["a", "b"]),
            PinEvent::Unpin(id("a")),
            pins(&["b"]),
        ),
        (
            "unpin an unpinned app",
            pins(&["a"]),
            PinEvent::Unpin(id("z")),
            pins(&["a"]),
        ),
        (
            "move right",
            pins(&["a", "b", "c"]),
            PinEvent::Move {
                app: id("a"),
                to: 2,
            },
            pins(&["b", "c", "a"]),
        ),
        (
            "move left",
            pins(&["a", "b", "c"]),
            PinEvent::Move {
                app: id("c"),
                to: 0,
            },
            pins(&["c", "a", "b"]),
        ),
        (
            "move past the end clamps",
            pins(&["a", "b"]),
            PinEvent::Move {
                app: id("a"),
                to: 7,
            },
            pins(&["b", "a"]),
        ),
        (
            "moving an unpinned app does nothing",
            pins(&["a", "b"]),
            PinEvent::Move {
                app: id("z"),
                to: 0,
            },
            pins(&["a", "b"]),
        ),
        (
            "replaced",
            pins(&["a"]),
            PinEvent::Replaced(pins(&["x", "y"])),
            pins(&["x", "y"]),
        ),
    ];
    for (name, before, event, want) in cases {
        assert_eq!(step(before, event), want, "{name}");
    }
}
