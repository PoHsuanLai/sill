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

#[test]
fn pins_under_an_old_id_become_the_renamed_app() {
    use crate::apps::{AppIndex, AppsEvent, step as index_step, test_app};
    let mut mail = test_app("org.quire.Mail", "Mail");
    mail.renamed_from = vec![id("mailo")];
    let mut clock = test_app("clock", "Clock");
    clock.renamed_from = vec![id("old-clock")];
    let installed = |apps| index_step(AppIndex::default(), AppsEvent::Scanned(apps));
    let index = installed(vec![
        mail.clone(),
        clock,
        test_app("mailo-legacy", "Legacy"),
    ]);
    let cases: Vec<(&str, DockPins, DockPins)> = vec![
        (
            "an old id is replaced in place",
            pins(&["a", "mailo", "b"]),
            pins(&["a", "org.quire.Mail", "b"]),
        ),
        (
            "old and new collapse to the first one's place",
            pins(&["mailo", "a", "org.quire.Mail"]),
            pins(&["org.quire.Mail", "a"]),
        ),
        (
            "new then old collapse to the first one's place",
            pins(&["org.quire.Mail", "a", "mailo"]),
            pins(&["org.quire.Mail", "a"]),
        ),
        (
            "two olds of one app collapse",
            pins(&["old-clock", "clock"]),
            pins(&["clock"]),
        ),
        (
            "an unknown id stays",
            pins(&["gone", "a"]),
            pins(&["gone", "a"]),
        ),
    ];
    for (name, stored, want) in cases {
        assert_eq!(stored.renamed(&index), want, "{name}");
    }
    let itself = installed(vec![mail, test_app("mailo", "Mailo")]);
    assert_eq!(
        pins(&["mailo"]).renamed(&itself),
        pins(&["mailo"]),
        "an installed id is not renamed away"
    );
}
