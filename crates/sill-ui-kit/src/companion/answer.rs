//! An answer as the card draws it.

use companion_wire::{AnswerBody, AnswerPhase, AnswerWire, NeedsYou, RefusalWire};
use docket_core::EntityRef;
use ds::components::companion::answer::action::{ActionRole, CardAction, CardActionId};
use ds::components::companion::answer::footer::CardFooter;
use ds::components::companion::answer::model::{AnswerView, Streaming, TextAnswer};
use ds::components::companion::answer::refused::{RefusedAnswer, RefusedView};
use ds::components::companion::draft::model::{DraftReply, PersonLine};
use ds::components::companion::proposed_event::model::ProposedEvent;
use ds::components::companion::replace::model::{ReplacePhase, ReplaceProposal};
use ds::components::content::avatar::{
    AvatarFace, AvatarShape, AvatarSize, AvatarTone, person_hue,
};
use ds::components::content::text_runs::TextLine;
use ds::stack::toast_hub::UndoToken;
use ds_core::vocab::{Availability, EffectMark};

use super::ViewCx;
use super::card::actions;
use super::footer::footer_view;
use super::form::form_view;
use super::plan::plan_view;
use super::words::{plain, refusal_words, when};

/// The card for `answer`; `prompt` is what the person asked (a plan's title, a form's).
pub fn answer_view(answer: &AnswerWire, prompt: &str, cx: &ViewCx<'_>) -> AnswerView {
    let footer = footer_view(&answer.footer);
    match &answer.body {
        AnswerBody::Text { lines } => AnswerView::Text(TextAnswer {
            body: lines
                .iter()
                .map(|line| TextLine::Plain(plain(line, cx)))
                .collect(),
            streaming: match answer.phase {
                AnswerPhase::Streaming | AnswerPhase::Thinking => Streaming::Arriving,
                AnswerPhase::NeedsYou(_)
                | AnswerPhase::Done
                | AnswerPhase::Failed
                | AnswerPhase::Cancelled => Streaming::Complete,
            },
            actions: needs_you_actions(&answer.phase),
            footer,
        }),
        AnswerBody::DraftReply {
            to,
            subject,
            body,
            actions: cards,
        } => AnswerView::DraftReply(DraftReply {
            to: to.iter().map(person).collect(),
            subject: plain(subject, cx),
            quoted: None,
            body: plain(body, cx),
            actions: actions(cards),
            footer,
        }),
        AnswerBody::ProposedEvent {
            title,
            when: range,
            people,
            actions: cards,
        } => AnswerView::ProposedEvent(ProposedEvent {
            title: title.clone(),
            when: when(range, cx.zone),
            place: None,
            people: people.iter().map(person).collect(),
            day: Vec::new(),
            actions: actions(cards),
            footer,
        }),
        AnswerBody::Plan(plan) => AnswerView::Plan(plan_view(plan, &answer.phase, prompt, footer)),
        AnswerBody::Replace {
            original,
            proposed,
            undo,
        } => AnswerView::Replace(ReplaceProposal {
            original: plain(original, cx),
            proposed: plain(proposed, cx),
            phase: match (undo, &answer.phase) {
                (Some(undo), _) => ReplacePhase::Applied {
                    undo: UndoToken(undo.0),
                },
                (None, AnswerPhase::Cancelled | AnswerPhase::Failed) => ReplacePhase::Discarded,
                (
                    None,
                    AnswerPhase::Thinking
                    | AnswerPhase::Streaming
                    | AnswerPhase::NeedsYou(_)
                    | AnswerPhase::Done,
                ) => ReplacePhase::Proposed,
            },
            footer,
        }),
        AnswerBody::Form(form) => AnswerView::Form(form_view(form, prompt, "Continue", footer)),
        AnswerBody::Refused(refusal) => refused(refusal, footer),
    }
}

/// A person on a draft or an event: the thing's title is their name, its subtitle their address.
fn person(entity: &EntityRef) -> PersonLine {
    let name = entity.title.value.clone();
    let address = entity.subtitle.value.clone();
    let initial = name
        .chars()
        .find(|c| c.is_alphanumeric())
        .map_or('?', |c| c.to_uppercase().next().unwrap_or(c));
    PersonLine {
        avatar: AvatarFace {
            initial,
            size: AvatarSize::Size22,
            tone: AvatarTone::Person(person_hue(if address.is_empty() {
                &name
            } else {
                &address
            })),
            shape: AvatarShape::Round,
        },
        name,
        address,
    }
}

/// A question or a confirmation the answer waits on has no card button of its own here: the
/// confirmation is the sheet's, a question's choices are buttons.
fn needs_you_actions(phase: &AnswerPhase) -> Vec<CardAction> {
    match phase {
        AnswerPhase::NeedsYou(NeedsYou::Question { choices, .. }) => choices
            .iter()
            .enumerate()
            .map(|(at, choice)| CardAction {
                id: CardActionId(choice.clone()),
                label: choice.clone(),
                role: if at == 0 {
                    ActionRole::Primary
                } else {
                    ActionRole::Secondary
                },
                effect: EffectMark::Read,
                keys: None,
                availability: Availability::Enabled,
            })
            .collect(),
        AnswerPhase::NeedsYou(NeedsYou::Confirm(_) | NeedsYou::Form(_))
        | AnswerPhase::Thinking
        | AnswerPhase::Streaming
        | AnswerPhase::Done
        | AnswerPhase::Failed
        | AnswerPhase::Cancelled => Vec::new(),
    }
}

/// The card of a refusal. A turn that met no model has no failure to show: the shell opens the
/// model picker, and the card is the footer alone with nothing in its body.
fn refused(refusal: &RefusalWire, footer: CardFooter) -> AnswerView {
    let (why, actions) = match refusal {
        RefusalWire::NeedsCloud(class) => (
            RefusedView::NeedsCloud {
                class: format!("{class:?}"),
            },
            vec![CardAction {
                id: CardActionId("allow_once".to_owned()),
                label: "Allow Once…".to_owned(),
                role: ActionRole::Secondary,
                effect: EffectMark::Read,
                keys: None,
                availability: Availability::Enabled,
            }],
        ),
        RefusalWire::NotAllowed(code) => (
            RefusedView::NotAllowed {
                what: refusal_words(&docket_core::CallRefusal::Denied(*code)),
            },
            Vec::new(),
        ),
        RefusalWire::NoWay { app } => (
            RefusedView::NoWay {
                app: app.as_str().to_owned(),
            },
            Vec::new(),
        ),
        RefusalWire::OverBudget(_) => (RefusedView::OverBudget, Vec::new()),
        RefusalWire::Failed(why) => (RefusedView::Failed { why: why.clone() }, Vec::new()),
        RefusalWire::NoModel => {
            return AnswerView::Text(TextAnswer {
                body: Vec::new(),
                streaming: Streaming::Complete,
                actions: Vec::new(),
                footer,
            });
        }
    };
    AnswerView::Refused(RefusedAnswer {
        why,
        actions,
        footer,
    })
}

/// `view` without its card buttons: where the buttons are drawn elsewhere (the launcher's pane
/// lists the row's actions with their keys), the card is the body and the footer alone.
pub fn without_actions(view: AnswerView) -> AnswerView {
    match view {
        AnswerView::Text(text) => AnswerView::Text(TextAnswer {
            actions: Vec::new(),
            ..text
        }),
        AnswerView::DraftReply(draft) => AnswerView::DraftReply(DraftReply {
            actions: Vec::new(),
            ..draft
        }),
        AnswerView::ProposedEvent(event) => AnswerView::ProposedEvent(ProposedEvent {
            actions: Vec::new(),
            ..event
        }),
        AnswerView::Refused(refused) => AnswerView::Refused(RefusedAnswer {
            actions: Vec::new(),
            ..refused
        }),
        view @ (AnswerView::Plan(_) | AnswerView::Replace(_) | AnswerView::Form(_)) => view,
    }
}
