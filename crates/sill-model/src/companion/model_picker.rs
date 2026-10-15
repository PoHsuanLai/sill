//! The turn that met no model: the companion says nothing about it, and the shell opens
//! Settings' model picker once for it.

use companion_wire::{AnswerBody, RefusalWire};
use prov::TaskId;

use super::command::{CompanionEffect, CompanionEvent};
use super::model::{AnswerFacts, Companion};

/// Whether an answer waits on the person choosing a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Picker {
    /// The answer needs nothing chosen, or was not asked by this shell.
    Unneeded,
    /// The person asked, and no model was set to answer.
    Wanted,
}

/// Whether the companion refused `body` for want of a model.
pub(super) fn lacks_model(body: &AnswerBody) -> bool {
    matches!(body, AnswerBody::Refused(RefusalWire::NoModel))
}

/// The answer an event is about, when it can change whether a picker is wanted.
pub(super) fn touched(event: &CompanionEvent) -> Option<&TaskId> {
    match event {
        CompanionEvent::Asked { task, .. } => Some(task),
        CompanionEvent::Answer(wire) => Some(&wire.task),
        _ => None,
    }
}

/// Whether `task`'s answer waits on a model. An answer whose words are not known was not asked
/// here (the companion's objects found on the bus at start), so no picker opens for it.
pub(super) fn picker_of(state: &Companion, task: &TaskId) -> Picker {
    match state.answers.get(task) {
        Some(answer) if asked_here(answer) && lacks_model(&answer.wire.body) => Picker::Wanted,
        _ => Picker::Unneeded,
    }
}

fn asked_here(answer: &AnswerFacts) -> bool {
    !answer.prompt.trim().is_empty()
}

/// What the move from `before` to `after` asks of the outside: the picker opens when it becomes
/// wanted, and not again while the answer is updated.
pub(super) fn opens(before: Picker, after: Picker) -> Vec<CompanionEffect> {
    match (before, after) {
        (Picker::Unneeded, Picker::Wanted) => vec![CompanionEffect::PickModel],
        (Picker::Unneeded | Picker::Wanted, _) => Vec::new(),
    }
}
