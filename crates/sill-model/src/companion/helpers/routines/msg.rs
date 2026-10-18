//! What tells the routines part things, and what it asks of the outside.

use companion_wire::{RoutineForm, RoutineReply, RoutinesView};
use docket_core::routine::RoutineId;
use prov::HelperId;

use super::draft::Repeat;

/// What the routines part is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutinesMsg {
    /// The list arrived.
    Listed(RoutinesView),
    /// A call to make a routine was answered.
    Replied(RoutineReply),
    /// The person asks to add a routine for this helper: the sheet opens.
    OpenSheet(HelperId),
    /// The person types what it is to do.
    EditGoal(String),
    /// The person chooses how often.
    SetRepeat(Repeat),
    /// The person types the time of day.
    EditTime(String),
    /// The person types the hours between runs.
    EditHours(String),
    /// The person closes the sheet without making anything.
    CloseSheet,
    /// The person confirms the sheet.
    Confirm,
    /// The person pauses a routine.
    Pause(RoutineId),
    /// The person resumes a routine.
    Resume(RoutineId),
    /// The person asks to remove a routine; they are asked to confirm.
    AskRemove(RoutineId),
    /// The person decides not to remove it.
    CancelRemove,
    /// The person confirms removing a routine they were asked about.
    Remove(RoutineId),
}

/// What the routines part asks the companion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutinesCommand {
    /// Read the list.
    List,
    /// Make a routine from what the person confirmed.
    Add(Box<RoutineForm>),
    /// Pause a routine.
    Pause(RoutineId),
    /// Resume a routine.
    Resume(RoutineId),
    /// Remove a routine.
    Remove(RoutineId),
}
