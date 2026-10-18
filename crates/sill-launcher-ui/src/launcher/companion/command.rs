//! What each companion act asks of the companion service.

use sill_launcher::{CompanionAct, HelperControl};
use sill_model::companion::{CompanionCommand, HelpersMsg, TerminalsMsg};

/// The command that does `act`. An ask is built from the panel's own context (`ask_input`), and
/// switching the panel's mode is the panel's, so neither is a command here.
pub fn command_of(act: CompanionAct) -> Option<CompanionCommand> {
    match act {
        CompanionAct::Ask { .. } | CompanionAct::Mode(_) => None,
        CompanionAct::Card { task, action } => Some(CompanionCommand::Card { task, action }),
        CompanionAct::Promote { task } => Some(CompanionCommand::Promote(task)),
        CompanionAct::Run { run, control } => Some(CompanionCommand::Run { run, control }),
        CompanionAct::Undo { scope } => Some(CompanionCommand::Undo(scope)),
        CompanionAct::Memory { item, verb } => Some(CompanionCommand::Memory { item, verb }),
        CompanionAct::Helper { helper, control } => {
            Some(CompanionCommand::Helpers(match control {
                HelperControl::Pause => HelpersMsg::Pause(helper),
                HelperControl::Resume => HelpersMsg::Resume(helper),
                // Remove always asks first; the card has the question.
                HelperControl::Remove => HelpersMsg::AskRemove(helper),
            }))
        }
        CompanionAct::NewHelper => Some(CompanionCommand::Helpers(HelpersMsg::NewHelper)),
        CompanionAct::BringBack { session } => Some(CompanionCommand::Terminals(
            TerminalsMsg::BringBack(session),
        )),
    }
}
