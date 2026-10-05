//! What the companion pane is showing, from the row the selection is on.

use companion_wire::{AnswerPhase, NeedsYou};
use prov::{RunId, TaskId};
use sill_launcher::{Item, Subject};
use sill_model::companion::Companion;

use super::keys::Selected;

/// Where the person is in companion mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pane {
    /// Typing a prompt: the selection is on the Ask row or another row.
    Composing,
    /// The selected answer is still being made (thinking or writing).
    Thinking(TaskId),
    /// The selected answer is in: its card is the pane.
    Showing(TaskId),
    /// The selected answer needs a few details: its form is the pane.
    Form(TaskId),
    /// The selected answer waits for the person's confirmation on the shell's own sheet: its
    /// card stays as it is, and the person answers on the sheet.
    Confirming(TaskId),
}

/// What the selected row is, as the keys machine reads it.
pub fn selected_of(item: Option<&Item>) -> Selected {
    match item.map(|item| &item.subject) {
        None => Selected::Nothing,
        Some(Subject::Answer(task)) => Selected::Answer(task.clone()),
        Some(Subject::Run(run)) => Selected::Run(run.clone()),
        Some(Subject::None) if is_ask(item) => Selected::Ask,
        Some(_) => Selected::Other,
    }
}

fn is_ask(item: Option<&Item>) -> bool {
    item.is_some_and(|item| item.id.0.starts_with("ask:"))
}

/// The pane for the selected row: an answer row shows its answer, by how far the answer is;
/// anything else is the prompt being composed.
pub fn pane_of(selected: &Selected, companion: &Companion) -> Pane {
    let Selected::Answer(task) = selected else {
        return Pane::Composing;
    };
    let Some(answer) = companion.answers.get(task) else {
        return Pane::Composing;
    };
    match &answer.wire.phase {
        AnswerPhase::Thinking | AnswerPhase::Streaming => Pane::Thinking(task.clone()),
        AnswerPhase::NeedsYou(NeedsYou::Form(_)) => Pane::Form(task.clone()),
        AnswerPhase::NeedsYou(NeedsYou::Confirm(_)) => Pane::Confirming(task.clone()),
        AnswerPhase::NeedsYou(NeedsYou::Question { .. })
        | AnswerPhase::Done
        | AnswerPhase::Failed
        | AnswerPhase::Cancelled => Pane::Showing(task.clone()),
    }
}

/// The run a selected row names, if it is a run.
pub fn run_of(selected: &Selected) -> Option<&RunId> {
    match selected {
        Selected::Run(run) => Some(run),
        Selected::Nothing | Selected::Ask | Selected::Answer(_) | Selected::Other => None,
    }
}
