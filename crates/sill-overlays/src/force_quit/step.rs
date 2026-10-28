//! The panel's transitions, pure: what the list, the Force Quit button, the confirmation sheet
//! and Escape do to a [`Chosen`], given the apps the desktop lists.

use sill_model::desktop::{ForceQuitRow, hung_first};

use super::model::Chosen;

/// What reaches the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum In {
    /// The desktop's list changed (or the panel opened on it).
    Listed,
    /// A row was pressed or the arrows reached it.
    Select(String),
    /// The Force Quit button was pressed.
    Ask,
    /// The confirmation sheet's Force Quit was pressed.
    Confirm,
    /// The confirmation sheet's Cancel was pressed.
    Decline,
    /// Escape, or a press away from the panel.
    Escape,
}

/// What the panel then does besides drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Out {
    Nothing,
    /// End this app's processes.
    Quit(String),
    /// Take the panel away.
    Close,
}

fn lists(rows: &[ForceQuitRow], app: &str) -> bool {
    rows.iter().any(|row| row.app == app)
}

/// `selection` after `input`, over `rows`.
pub(super) fn advance(selection: Chosen, input: In, rows: &[ForceQuitRow]) -> (Chosen, Out) {
    match (selection.confirming.clone(), input) {
        (_, In::Listed) => (settled(selection, rows), Out::Nothing),
        (None, In::Select(app)) if lists(rows, &app) => (
            Chosen {
                app: Some(app),
                ..selection
            },
            Out::Nothing,
        ),
        (None, In::Ask) => match selection.app.clone().filter(|app| lists(rows, app)) {
            Some(app) => (
                Chosen {
                    confirming: Some(app),
                    ..selection
                },
                Out::Nothing,
            ),
            None => (selection, Out::Nothing),
        },
        (Some(app), In::Confirm) => (
            Chosen {
                app: None,
                confirming: None,
            },
            Out::Quit(app),
        ),
        (Some(_), In::Decline | In::Escape) => (
            Chosen {
                confirming: None,
                ..selection
            },
            Out::Nothing,
        ),
        (None, In::Escape) => (selection, Out::Close),
        _ => (selection, Out::Nothing),
    }
}

/// `selection` against a list that changed: an app that left is neither selected nor asked
/// about, and a hung app starts selected when nothing is.
fn settled(selection: Chosen, rows: &[ForceQuitRow]) -> Chosen {
    let app = selection
        .app
        .filter(|app| lists(rows, app))
        .or_else(|| hung_first(rows).map(str::to_owned));
    let confirming = selection.confirming.filter(|asked| lists(rows, asked));
    Chosen { app, confirming }
}
