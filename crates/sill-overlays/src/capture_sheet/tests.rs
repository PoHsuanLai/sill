//! The stage as a table: pure.

use sill_model::capture_pick::{Ask, AskId, Asking};

use super::stage::{Mapped, Stage, StageIn, advance};

fn asking(id: u64) -> Asking {
    Asking {
        id: AskId(id),
        ask: Ask::from_wire("Meet", 3, 0, &[1]).expect("a valid ask"), // test-only
    }
}

#[test]
fn the_stage_follows_the_request_and_plays_the_exit() {
    type Case = (&'static str, Stage, StageIn, Stage, Mapped);
    let cases: Vec<Case> = vec![
        (
            "a request maps the surface",
            Stage::Empty,
            StageIn::Want(Some(asking(1))),
            Stage::Up(asking(1)),
            Mapped::Yes,
        ),
        (
            "its going starts the exit",
            Stage::Up(asking(1)),
            StageIn::Want(None),
            Stage::Leaving(asking(1)),
            Mapped::Yes,
        ),
        (
            "the exit's settle unmaps",
            Stage::Leaving(asking(1)),
            StageIn::Settled,
            Stage::Empty,
            Mapped::No,
        ),
        (
            "the next request replaces a leaving one",
            Stage::Leaving(asking(1)),
            StageIn::Want(Some(asking(2))),
            Stage::Up(asking(2)),
            Mapped::Yes,
        ),
        (
            "nothing wanted, nothing shown",
            Stage::Empty,
            StageIn::Want(None),
            Stage::Empty,
            Mapped::No,
        ),
        (
            "a settle with the request up changes nothing",
            Stage::Up(asking(1)),
            StageIn::Settled,
            Stage::Up(asking(1)),
            Mapped::Yes,
        ),
    ];
    for (name, from, input, want, mapped) in cases {
        let next = advance(from, input);
        assert_eq!(next, want, "{name}");
        assert_eq!(next.mapped(), mapped, "{name}");
    }
}
