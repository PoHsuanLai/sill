//! The stage as a table: pure.

use super::model::{Mapped, Pointer, SheetStage, StageIn};
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

#[test]
fn a_sheet_leaves_with_its_answer_and_hands_it_over_once_settled() {
    use super::exit::{ExitIn, ExitOut, SheetExit};
    type Exit = SheetExit<u8>;
    type Case = (&'static str, Exit, ExitIn<u8>, Exit, ExitOut<u8>);
    #[rustfmt::skip]
    let cases: &[Case] = &[
        ("an answer starts the exit",            Exit::Open,       ExitIn::Leave(1), Exit::Leaving(1), ExitOut::Left),
        ("a second answer while leaving waits",  Exit::Leaving(1), ExitIn::Leave(2), Exit::Leaving(1), ExitOut::Nothing),
        ("the settle hands the answer over",     Exit::Leaving(1), ExitIn::Settled,  Exit::Leaving(1), ExitOut::Finished(1)),
        ("a settle while up is not an exit",     Exit::Open,       ExitIn::Settled,  Exit::Open,       ExitOut::Nothing),
        ("shown again brings the sheet back",    Exit::Leaving(1), ExitIn::Reshown,  Exit::Open,       ExitOut::Nothing),
        ("shown again while up changes nothing", Exit::Open,       ExitIn::Reshown,  Exit::Open,       ExitOut::Nothing),
    ];
    for (name, from, input, want, out) in cases {
        assert_eq!(from.advance(*input), (*want, *out), "{name}");
    }
    assert_eq!(Exit::Open.shown(), Shown::Visible);
    assert_eq!(Exit::Leaving(1).shown(), Shown::Hidden);
}

#[test]
fn a_sheet_takes_the_pointer_only_while_it_is_up() {
    use super::exit::SheetExit;
    assert_eq!(Stage::Empty.pointer(), Pointer::Passed);
    assert_eq!(Stage::Up(1).pointer(), Pointer::Taken);
    assert_eq!(
        Stage::Leaving(1).pointer(),
        Pointer::Passed,
        "an exit lets clicks through"
    );
    assert_eq!(SheetExit::<u8>::Open.pointer(), Pointer::Taken);
    assert_eq!(SheetExit::Leaving(1u8).pointer(), Pointer::Passed);
}
