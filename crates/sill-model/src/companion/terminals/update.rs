//! The transition: `(model, message) -> (model, commands)`.

use companion_wire::BroughtBack;

use super::model::{RefusedBringBack, TerminalsModel};
use super::msg::{TerminalsCommand, TerminalsMsg};

/// `model` after `msg`, and what the panel asks the companion.
pub fn update(model: TerminalsModel, msg: TerminalsMsg) -> (TerminalsModel, Vec<TerminalsCommand>) {
    match msg {
        TerminalsMsg::Changed => (model, vec![TerminalsCommand::List]),
        TerminalsMsg::Listed(sessions) => (
            TerminalsModel {
                sessions,
                refused: None,
                ..model
            },
            Vec::new(),
        ),
        TerminalsMsg::Unreached => (model, Vec::new()),
        TerminalsMsg::BringBack(session) => (
            TerminalsModel {
                refused: None,
                ..model
            },
            vec![TerminalsCommand::BringBack(session)],
        ),
        TerminalsMsg::Answered(_, BroughtBack::Session(session)) => (
            TerminalsModel {
                brought_back: Some(session),
                refused: None,
                ..model
            },
            vec![TerminalsCommand::List],
        ),
        TerminalsMsg::Answered(session, BroughtBack::Refused(sentence)) => (
            TerminalsModel {
                refused: Some(RefusedBringBack { session, sentence }),
                ..model
            },
            Vec::new(),
        ),
    }
}
