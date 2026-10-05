//! What each companion act asks of the companion service.

use sill_launcher::CompanionAct;
use sill_model::companion::CompanionCommand;

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
    }
}
