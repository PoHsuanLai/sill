//! What the routines part of the helpers screen holds.

use companion_wire::RoutineLine;
use docket_core::routine::RoutineId;
use prov::HelperId;

use super::draft::RoutineDraft;

/// The sheet where a routine is confirmed before it is made.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Sheet {
    /// No sheet.
    #[default]
    Closed,
    /// A routine being filled in; nothing is made until the person confirms.
    Adding(RoutineDraft),
}

/// Whether the person was asked to confirm removing a routine.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RoutineRemoval {
    /// No question is open.
    #[default]
    Unasked,
    /// Asked "Remove it?" about this routine and waiting for the answer.
    Asked(RoutineId),
}

/// Everything the routines part shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoutinesModel {
    /// One line per routine, in the order the companion lists them.
    pub lines: Vec<RoutineLine>,
    /// The sheet that makes a routine.
    pub sheet: Sheet,
    /// The question about removing a routine.
    pub removal: RoutineRemoval,
    /// A sentence about what was refused or is missing from the sheet, until the person does the
    /// next thing.
    pub notice: Option<String>,
}

impl RoutinesModel {
    /// The routines of the helper `helper`.
    pub fn of_helper<'a>(
        &'a self,
        helper: &'a HelperId,
    ) -> impl Iterator<Item = &'a RoutineLine> + 'a {
        self.lines.iter().filter(move |line| line.helper == *helper)
    }
}
