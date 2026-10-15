use sill_model::desktop::{ForceQuitRow, Standing};

use super::model::{Chosen, Confirming};
use super::stage::{Mapped, Stage, StageIn, advance as stage};
use super::step::{In, Out, advance};
use sill_ui_kit::links::{ForceQuitOpening, ForceQuitSerial};

fn row(app: &str, standing: Standing) -> ForceQuitRow {
    ForceQuitRow {
        app: app.to_owned(),
        standing,
    }
}

fn rows() -> Vec<ForceQuitRow> {
    vec![
        row("mail", Standing::Responding),
        row("term", Standing::NotResponding),
        row("notes", Standing::Responding),
    ]
}

fn selected(app: &str) -> Chosen {
    Chosen {
        app: Some(app.to_owned()),
        confirming: Confirming::No,
    }
}

fn asking(app: &str) -> Chosen {
    Chosen {
        app: Some(app.to_owned()),
        confirming: Confirming::App(app.to_owned()),
    }
}

#[test]
fn the_panel_selects_asks_confirms_and_leaves() {
    type Case = (&'static str, Chosen, In, Chosen, Out);
    let cases: Vec<Case> = vec![
        (
            "a hung app starts selected",
            Chosen::default(),
            In::Listed,
            selected("term"),
            Out::Nothing,
        ),
        (
            "a row pressed is selected",
            selected("term"),
            In::Select("notes".to_owned()),
            selected("notes"),
            Out::Nothing,
        ),
        (
            "an app the list lacks cannot be selected",
            selected("term"),
            In::Select("ghost".to_owned()),
            selected("term"),
            Out::Nothing,
        ),
        (
            "Force Quit asks about the selected app",
            selected("notes"),
            In::Ask,
            asking("notes"),
            Out::Nothing,
        ),
        (
            "Force Quit with nothing selected asks nothing",
            Chosen::default(),
            In::Ask,
            Chosen::default(),
            Out::Nothing,
        ),
        (
            "the list is shut while the question is up",
            asking("notes"),
            In::Select("mail".to_owned()),
            asking("notes"),
            Out::Nothing,
        ),
        (
            "confirming ends the app and clears the selection",
            asking("notes"),
            In::Confirm,
            Chosen::default(),
            Out::Quit("notes".to_owned()),
        ),
        (
            "declining keeps the selection",
            asking("notes"),
            In::Decline,
            selected("notes"),
            Out::Nothing,
        ),
        (
            "Escape on the question answers it, it does not close the panel",
            asking("notes"),
            In::Escape,
            selected("notes"),
            Out::Nothing,
        ),
        (
            "Escape on the list closes the panel",
            selected("notes"),
            In::Escape,
            selected("notes"),
            Out::Close,
        ),
        (
            "an app that left while asked about is neither selected nor asked",
            asking("ghost"),
            In::Listed,
            Chosen {
                app: Some("term".to_owned()),
                confirming: Confirming::No,
            },
            Out::Nothing,
        ),
        (
            "a selected app that is still listed stays selected",
            selected("notes"),
            In::Listed,
            selected("notes"),
            Out::Nothing,
        ),
        (
            "confirm without a question does nothing",
            selected("notes"),
            In::Confirm,
            selected("notes"),
            Out::Nothing,
        ),
    ];
    for (name, from, input, want, out) in cases {
        assert_eq!(advance(from, input, &rows()), (want, out), "{name}");
    }
}

fn opening(serial: u64) -> ForceQuitOpening {
    ForceQuitOpening {
        serial: ForceQuitSerial(serial),
        output: shell_host::OutputId(1),
    }
}

#[test]
fn the_stage_follows_the_link_and_plays_the_exit() {
    type Case = (&'static str, Stage, StageIn, Stage, Mapped);
    let cases: Vec<Case> = vec![
        (
            "opening maps the surface",
            Stage::Empty,
            StageIn::Want(Some(opening(1))),
            Stage::Up(opening(1)),
            Mapped::Yes,
        ),
        (
            "closing starts the exit",
            Stage::Up(opening(1)),
            StageIn::Want(None),
            Stage::Leaving(opening(1)),
            Mapped::Yes,
        ),
        (
            "the exit's settle unmaps",
            Stage::Leaving(opening(1)),
            StageIn::Settled,
            Stage::Empty,
            Mapped::No,
        ),
        (
            "the next opening replaces a leaving one",
            Stage::Leaving(opening(1)),
            StageIn::Want(Some(opening(2))),
            Stage::Up(opening(2)),
            Mapped::Yes,
        ),
        (
            "a settle with the panel up changes nothing",
            Stage::Up(opening(1)),
            StageIn::Settled,
            Stage::Up(opening(1)),
            Mapped::Yes,
        ),
    ];
    for (name, from, input, want, mapped) in cases {
        let next = stage(from, input);
        assert_eq!(next, want, "{name}");
        assert_eq!(next.mapped(), mapped, "{name}");
    }
}
