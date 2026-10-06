//! The stage as a table: pure.

use sill_model::voice::ConsentSheet::{Asking, Closed};

use super::stage::{Mapped, Stage, StageIn, advance};

#[test]
fn the_sheet_enters_when_asked_and_leaves_through_its_exit() {
    type Case = (&'static str, Stage, StageIn, Stage, Mapped);
    const CASES: &[Case] = &[
        (
            "nothing asked",
            Stage::Empty,
            StageIn::Want(Closed),
            Stage::Empty,
            Mapped::No,
        ),
        (
            "asked",
            Stage::Empty,
            StageIn::Want(Asking),
            Stage::Up,
            Mapped::Yes,
        ),
        (
            "still asked",
            Stage::Up,
            StageIn::Want(Asking),
            Stage::Up,
            Mapped::Yes,
        ),
        (
            "answered",
            Stage::Up,
            StageIn::Want(Closed),
            Stage::Leaving,
            Mapped::Yes,
        ),
        (
            "exit settled",
            Stage::Leaving,
            StageIn::Settled,
            Stage::Empty,
            Mapped::No,
        ),
        (
            "asked again mid-exit",
            Stage::Leaving,
            StageIn::Want(Asking),
            Stage::Up,
            Mapped::Yes,
        ),
        (
            "a settle with nothing up",
            Stage::Empty,
            StageIn::Settled,
            Stage::Empty,
            Mapped::No,
        ),
        (
            "a settle while up is not its exit",
            Stage::Up,
            StageIn::Settled,
            Stage::Up,
            Mapped::Yes,
        ),
    ];
    for (name, from, input, want, mapped) in CASES {
        let next = advance(*from, *input);
        assert_eq!(next, *want, "{name}");
        assert_eq!(next.mapped(), *mapped, "{name}");
    }
}
