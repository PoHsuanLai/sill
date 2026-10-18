//! The routines part's one transition: `(model, message) -> (model, commands)`.

use companion_wire::{ProposalId, RoutineProposal, RoutineReply, RoutinesView};

use super::draft::{Repeat, RoutineDraft};
use super::model::{RoutineRemoval, RoutinesModel, Sheet};
use super::msg::{RoutinesCommand, RoutinesMsg};

/// `model` after `msg`, and what the routines part asks the companion.
pub fn update(model: RoutinesModel, msg: RoutinesMsg) -> (RoutinesModel, Vec<RoutinesCommand>) {
    match msg {
        RoutinesMsg::Listed(view) => (listed(model, view), Vec::new()),
        RoutinesMsg::Replied(reply) => replied(model, reply),
        RoutinesMsg::Proposed(proposals) => (RoutinesModel { proposals, ..model }, Vec::new()),
        RoutinesMsg::AcceptProposal(id) => {
            let known = model.proposals.iter().any(|proposal| proposal.id == id);
            let sent = if known {
                vec![RoutinesCommand::AcceptProposal(id)]
            } else {
                Vec::new()
            };
            (model, sent)
        }
        RoutinesMsg::DeclineProposal(id) => (
            RoutinesModel {
                proposals: without(model.proposals, id),
                ..model
            },
            vec![RoutinesCommand::DeclineProposal(id)],
        ),
        RoutinesMsg::Accepted(reply) => accepted(model, reply),
        RoutinesMsg::OpenSheet(helper) => (
            RoutinesModel {
                sheet: Sheet::Adding(RoutineDraft::blank(helper)),
                notice: None,
                ..model
            },
            Vec::new(),
        ),
        RoutinesMsg::EditGoal(goal) => (edited(model, |d| RoutineDraft { goal, ..d }), Vec::new()),
        RoutinesMsg::SetRepeat(repeat) => (edited(model, |d| repeated(d, repeat)), Vec::new()),
        RoutinesMsg::EditTime(time) => (edited(model, |d| RoutineDraft { time, ..d }), Vec::new()),
        RoutinesMsg::EditHours(hours) => {
            (edited(model, |d| RoutineDraft { hours, ..d }), Vec::new())
        }
        RoutinesMsg::CloseSheet => (
            RoutinesModel {
                sheet: Sheet::Closed,
                notice: None,
                ..model
            },
            Vec::new(),
        ),
        RoutinesMsg::Confirm => confirm(model),
        RoutinesMsg::Pause(id) => (model, vec![RoutinesCommand::Pause(id)]),
        RoutinesMsg::Resume(id) => (model, vec![RoutinesCommand::Resume(id)]),
        RoutinesMsg::AskRemove(id) => (
            RoutinesModel {
                removal: RoutineRemoval::Asked(id),
                ..model
            },
            Vec::new(),
        ),
        RoutinesMsg::CancelRemove => (
            RoutinesModel {
                removal: RoutineRemoval::Unasked,
                ..model
            },
            Vec::new(),
        ),
        RoutinesMsg::Remove(id) => remove(model, id),
    }
}

/// The list arrived: keep it, and drop a question about a routine that has gone.
fn listed(model: RoutinesModel, view: RoutinesView) -> RoutinesModel {
    let removal = match model.removal {
        RoutineRemoval::Asked(id) if !view.routines.iter().any(|line| line.id == id) => {
            RoutineRemoval::Unasked
        }
        other => other,
    };
    RoutinesModel {
        lines: view.routines,
        removal,
        ..model
    }
}

/// A call to make a routine was answered: the sheet closes, or says why it was refused.
fn replied(model: RoutinesModel, reply: RoutineReply) -> (RoutinesModel, Vec<RoutinesCommand>) {
    match reply {
        RoutineReply::Refused(sentence) => (
            RoutinesModel {
                notice: Some(sentence),
                ..model
            },
            Vec::new(),
        ),
        RoutineReply::Done | RoutineReply::Made(_) => (
            RoutinesModel {
                sheet: Sheet::Closed,
                notice: None,
                ..model
            },
            vec![RoutinesCommand::List],
        ),
    }
}

/// A suggestion was accepted, or refused with a sentence that is shown. The lists are read again
/// either way, since the suggestion may be gone.
fn accepted(model: RoutinesModel, reply: RoutineReply) -> (RoutinesModel, Vec<RoutinesCommand>) {
    let notice = match reply {
        RoutineReply::Refused(sentence) => Some(sentence),
        RoutineReply::Done | RoutineReply::Made(_) => None,
    };
    (
        RoutinesModel { notice, ..model },
        vec![RoutinesCommand::List, RoutinesCommand::ListProposals],
    )
}

/// `proposals` without the one called `id`.
fn without(proposals: Vec<RoutineProposal>, id: ProposalId) -> Vec<RoutineProposal> {
    proposals
        .into_iter()
        .filter(|proposal| proposal.id != id)
        .collect()
}

/// `draft` repeating as `repeat`.
fn repeated(draft: RoutineDraft, repeat: Repeat) -> RoutineDraft {
    RoutineDraft { repeat, ..draft }
}

/// `model` with the draft on its sheet changed by `change`; nothing when no sheet is open.
fn edited(
    model: RoutinesModel,
    change: impl FnOnce(RoutineDraft) -> RoutineDraft,
) -> RoutinesModel {
    let sheet = match model.sheet {
        Sheet::Adding(draft) => Sheet::Adding(change(draft)),
        Sheet::Closed => Sheet::Closed,
    };
    RoutinesModel {
        sheet,
        notice: None,
        ..model
    }
}

/// Confirm the sheet: what is missing is said, and nothing is sent.
fn confirm(model: RoutinesModel) -> (RoutinesModel, Vec<RoutinesCommand>) {
    let Sheet::Adding(draft) = &model.sheet else {
        return (model, Vec::new());
    };
    match draft.form() {
        Ok(form) => (
            RoutinesModel {
                notice: None,
                ..model
            },
            vec![RoutinesCommand::Add(Box::new(form))],
        ),
        Err(fault) => (
            RoutinesModel {
                notice: Some(fault.to_string()),
                ..model
            },
            Vec::new(),
        ),
    }
}

/// Remove a routine the person was asked about; a removal nobody asked about is not done.
fn remove(
    model: RoutinesModel,
    id: docket_core::routine::RoutineId,
) -> (RoutinesModel, Vec<RoutinesCommand>) {
    if model.removal != RoutineRemoval::Asked(id) {
        return (model, Vec::new());
    }
    (
        RoutinesModel {
            removal: RoutineRemoval::Unasked,
            ..model
        },
        vec![RoutinesCommand::Remove(id)],
    )
}
