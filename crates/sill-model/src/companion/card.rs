//! What a card button is doing: which action a button calls, when its press has ended, and which
//! handles of an answer still wait for the text the router shows for them.

use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, CardWire, StepWireState};
use std::collections::BTreeSet;

use docket_core::{ActionRef, CardActionId, Handle, Reveal, ValidManifest};

use super::model::{AnswerFacts, Busy, Opener};

/// The buttons an answer's body offers.
pub fn cards_of(body: &AnswerBody) -> &[CardWire] {
    match body {
        AnswerBody::DraftReply { actions, .. } | AnswerBody::ProposedEvent { actions, .. } => {
            actions
        }
        _ => &[],
    }
}

/// The action `action`'s button calls, if `answer` offers that button.
pub fn card_call(answer: &AnswerFacts, action: &CardActionId) -> Option<ActionRef> {
    cards_of(&answer.wire.body)
        .iter()
        .find(|card| card.id == *action)
        .map(|card| card.call.action.clone())
}

/// The button a person means by "open it in its app": the first one that only reads.
pub fn promote_card(answer: &AnswerFacts) -> Option<CardActionId> {
    cards_of(&answer.wire.body)
        .iter()
        .find(|card| card.effect == prov::Effect::Read)
        .map(|card| card.id.clone())
}

/// `busy` after the answer became `wire`: a press ends when the answer's plan has its step and
/// the step is no longer pending or running, or when the task itself failed or was stopped.
pub(super) fn settled(busy: Busy, wire: &AnswerWire) -> Busy {
    let Busy::Acting { step, .. } = &busy else {
        return busy;
    };
    let stopped = matches!(wire.phase, AnswerPhase::Failed | AnswerPhase::Cancelled);
    let ended = match &wire.body {
        AnswerBody::Plan(plan) => {
            plan.steps
                .iter()
                .filter(|plan_step| plan_step.action == *step)
                .all(|plan_step| {
                    !matches!(
                        plan_step.state,
                        StepWireState::Pending | StepWireState::Running
                    )
                })
                && plan.steps.iter().any(|plan_step| plan_step.action == *step)
        }
        _ => false,
    };
    if stopped || ended { Busy::Idle } else { busy }
}

/// The handles `answer` shows that the router has not yet been asked to show, in the order they
/// appear.
pub fn unresolved_handles(answer: &AnswerFacts) -> Vec<Handle> {
    let mut found: Vec<Handle> = Vec::new();
    let mut note = |text: &Reveal<String>| {
        if let Reveal::Handle(handle) = text
            && !answer.shown.contains_key(handle)
            && !found.contains(handle)
        {
            found.push(*handle);
        }
    };
    match &answer.wire.body {
        AnswerBody::Text { lines } => lines.iter().for_each(&mut note),
        AnswerBody::DraftReply { subject, body, .. } => {
            note(subject);
            note(body);
        }
        AnswerBody::Replace {
            original, proposed, ..
        } => {
            note(original);
            note(proposed);
        }
        AnswerBody::ProposedEvent { .. }
        | AnswerBody::Plan(_)
        | AnswerBody::Form(_)
        | AnswerBody::Refused(_) => {}
    }
    found
}

/// The kinds of thing each of `manifests` opens: those whose app declares the kind's open
/// action (`docket_core::open_name`).
pub fn openers_of(manifests: &[ValidManifest]) -> BTreeSet<Opener> {
    manifests
        .iter()
        .map(ValidManifest::manifest)
        .flat_map(|manifest| {
            manifest
                .entities
                .iter()
                .filter(|entity| manifest.open_of(&entity.kind).is_some())
                .map(|entity| Opener {
                    app: manifest.app.clone(),
                    kind: entity.kind.clone(),
                })
        })
        .collect()
}
