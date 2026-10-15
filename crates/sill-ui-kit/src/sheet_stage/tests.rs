//! The stage as a table: pure.

use super::model::{Mapped, SheetStage, StageIn};
use super::step::advance;
use ds::prelude::Shown;

type Stage = SheetStage<u8>;

#[test]
fn the_stage_follows_the_wanted_value_and_plays_the_exit() {
    #[rustfmt::skip]
    let cases: &[(&str, Stage, StageIn<u8>, Stage, Mapped)] = &[
        ("a value maps the surface",            Stage::Empty,      StageIn::Want(Some(1)), Stage::Up(1),      Mapped::Yes),
        ("its going starts the exit",           Stage::Up(1),      StageIn::Want(None),    Stage::Leaving(1), Mapped::Yes),
        ("the exit's settle unmaps",            Stage::Leaving(1), StageIn::Settled,       Stage::Empty,      Mapped::No),
        ("the next value replaces a leaving one", Stage::Leaving(1), StageIn::Want(Some(2)), Stage::Up(2),    Mapped::Yes),
        ("the next value replaces one that is up", Stage::Up(1),   StageIn::Want(Some(2)), Stage::Up(2),     Mapped::Yes),
        ("the same value again stays up",       Stage::Up(1),      StageIn::Want(Some(1)), Stage::Up(1),      Mapped::Yes),
        ("nothing wanted, nothing shown",       Stage::Empty,      StageIn::Want(None),    Stage::Empty,      Mapped::No),
        ("nothing wanted while leaving waits",  Stage::Leaving(1), StageIn::Want(None),    Stage::Leaving(1), Mapped::Yes),
        ("a settle with the value up changes nothing", Stage::Up(1), StageIn::Settled,     Stage::Up(1),      Mapped::Yes),
        ("a settle with nothing shown changes nothing", Stage::Empty, StageIn::Settled,    Stage::Empty,      Mapped::No),
    ];
    for (name, from, input, want, mapped) in cases {
        let next = advance(from.clone(), input.clone());
        assert_eq!(&next, want, "{name}");
        assert_eq!(next.mapped(), *mapped, "{name}");
    }
}

/// A case: its name, the stage, and what is drawn.
type DrawnCase = (&'static str, Stage, Option<(u8, Shown)>);

#[test]
fn a_value_is_drawn_visible_while_up_and_hidden_while_leaving() {
    #[rustfmt::skip]
    let cases: &[DrawnCase] = &[
        ("empty draws nothing",  Stage::Empty,      None),
        ("up is visible",        Stage::Up(1),      Some((1, Shown::Visible))),
        ("leaving is hidden",    Stage::Leaving(1), Some((1, Shown::Hidden))),
    ];
    for (name, stage, want) in cases {
        let drawn = stage.drawn().map(|(value, shown)| (*value, shown));
        assert_eq!(&drawn, want, "{name}");
    }
}
