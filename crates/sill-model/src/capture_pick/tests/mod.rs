mod ask;
mod color;
mod desktop;
mod eyedrop;
mod permission;
mod reply;
mod step;

use super::*;

pub(super) fn candidates() -> Candidates {
    Candidates {
        outputs: vec![
            OutputCandidate {
                name: "DP-1".to_owned(),
                title: "Studio Display".to_owned(),
            },
            OutputCandidate {
                name: "eDP-1".to_owned(),
                title: "Built-in".to_owned(),
            },
        ],
        windows: vec![
            WindowCandidate {
                identifier: "w-1".to_owned(),
                title: "Notes".to_owned(),
                app: "Notes".to_owned(),
            },
            WindowCandidate {
                identifier: "w-2".to_owned(),
                title: "Inbox".to_owned(),
                app: "Mail".to_owned(),
            },
        ],
    }
}

pub(super) fn ask(kinds: u32, multiple: u32, cursors: &[u32]) -> Ask {
    Ask::from_wire("Meet", kinds, multiple, cursors).expect("a valid ask") // test-only
}

pub(super) fn output(name: &str) -> Source {
    Source::Output(name.to_owned())
}

pub(super) fn window(id: &str) -> Source {
    Source::Window(id.to_owned())
}
