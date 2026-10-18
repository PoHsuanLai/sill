//! The pane: the card of the helper whose row is selected, or the blank card of a helper not made
//! yet. It asks the companion to open the card when it appears, and draws the card once the model
//! has it. Pause, Resume and Remove are the row's actions, which the pane lists under the card;
//! Remove asks here, in the card, before anything goes.

use dioxus::prelude::*;
use ds::components::content::label::{LabelRole, LabelStyle};
use ds::components::controls::button_model::ButtonRole;
use ds::prelude::*;
use prov::{HelperId, SpaceId};
use sill_model::companion::{
    Draft, HelpersModel, HelpersMsg, Open, Removal, WAITING_BADGE, remove_question,
};
use sill_services::CompanionService;
use sill_ui_kit::hooks::use_service;

use super::agent::AppLine;
use super::form::Fields;
use super::jobs::{GiveJob, Jobs, Latest};
use super::routines::Routines;
use super::say::{Say, use_say};
use super::sight::SightLine;

/// The card of `helper`, or of a new helper when `helper` is `None`. The card is keyed by the
/// helper, so the card of another helper starts afresh.
#[component]
pub(crate) fn HelperCardPane(helper: Option<HelperId>) -> Element {
    let companion = use_service::<CompanionService>();
    let opener = use_say();
    use_effect(use_reactive!(|helper| {
        opener.msg(match helper {
            Some(id) => HelpersMsg::OpenCard(id),
            None => HelpersMsg::NewHelper,
        });
    }));
    let model = companion.read().helpers.clone();
    let key = helper
        .as_ref()
        .map_or_else(|| "new".to_owned(), ToString::to_string);
    let body = match shown(helper.as_ref(), &model.open) {
        Shown::Ready => rsx! {
            CardBody { key: "{key}", model, helper }
        },
        Shown::Reading => rsx! {
            Label { text: "Reading\u{2026}", role: LabelRole::Secondary }
        },
    };
    rsx! {
        div { class: "sill-helper-card", {body} }
    }
}

/// Whether the model has the card the pane is for.
enum Shown {
    /// It does.
    Ready,
    /// It was asked for and has not arrived.
    Reading,
}

fn shown(helper: Option<&HelperId>, open: &Open) -> Shown {
    match (helper, open) {
        (Some(id), Open::Card(card)) if card.view.line.id == *id => Shown::Ready,
        (None, Open::New(_)) => Shown::Ready,
        (Some(_) | None, _) => Shown::Reading,
    }
}

/// The card, with the words the person is typing held here until they commit them.
#[component]
fn CardBody(model: HelpersModel, helper: Option<HelperId>) -> Element {
    let say = use_say();
    let seed = model
        .draft()
        .cloned()
        .unwrap_or_else(|| Draft::blank(SpaceId::desktop()));
    let name = use_signal(|| seed.name.clone());
    let role = use_signal(|| seed.role.clone());
    let Some(draft) = model.draft().cloned() else {
        return rsx! {};
    };
    let save = save_button(helper.as_ref(), name, role, &say);
    rsx! {
        {header(&model, helper.as_ref())}
        if let Open::Card(open) = &model.open {
            AppLine { view: open.view.clone() }
        }
        Fields {
            draft,
            apps: model.apps.clone(),
            models: model.models.clone(),
            automatic: model.automatic.clone(),
            helper: helper.clone(),
            name,
            role,
        }
        SightLine {}
        div { class: "sill-helper-buttons", {save} }
        if let Some(sentence) = &model.notice {
            p { class: "sill-helper-notice", role: "status", "{sentence}" }
        }
        if let Some(id) = &helper {
            {removal_question(&model, id, &say)}
            Jobs { model: model.clone() }
            Routines { model: model.clone(), helper: id.clone() }
            Latest { model: model.clone() }
            GiveJob { model }
        }
    }
}

/// The name and what the helper is doing, with the badge when it needs the person.
fn header(model: &HelpersModel, helper: Option<&HelperId>) -> Element {
    let Some(line) = helper.and_then(|id| model.line(id)) else {
        return rsx! {
            Label { text: "New Helper", style: LabelStyle::Title }
        };
    };
    rsx! {
        div { class: "sill-helper-head",
            Label { text: line.name.as_str().to_owned(), style: LabelStyle::Title }
            Label { text: line.doing.clone(), role: LabelRole::Secondary }
            if line.needs_you.is_some() {
                Label { text: WAITING_BADGE, style: LabelStyle::Caption }
            }
        }
    }
}

/// Save (an existing card) or Create (a new one). The words typed are committed first, in order,
/// so the save sees them whatever the field did on its way out.
fn save_button(
    helper: Option<&HelperId>,
    name: Signal<String>,
    role: Signal<String>,
    say: &Say,
) -> Element {
    let say = say.clone();
    let label = match helper {
        Some(_) => "Save",
        None => "Create",
    };
    rsx! {
        Button {
            label,
            common: Common { id: Some("helper-save".to_owned()), ..Common::default() },
            onclick: move |_| {
                say.msg(HelpersMsg::EditName(name()));
                say.msg(HelpersMsg::EditRole(role()));
                say.msg(HelpersMsg::Save);
            },
        }
    }
}

/// "Remove Research? Its finished jobs stay in your history.", with its two answers, while the
/// person was asked.
fn removal_question(model: &HelpersModel, id: &HelperId, say: &Say) -> Element {
    if model.removal != Removal::Asked(id.clone()) {
        return rsx! {};
    }
    let name = model
        .line(id)
        .map(|line| line.name.as_str().to_owned())
        .unwrap_or_default();
    let question = remove_question(&name);
    let (keep, remove, helper) = (say.clone(), say.clone(), id.clone());
    rsx! {
        div { class: "sill-helper-ask", role: "alert",
            p { "{question}" }
            div { class: "sill-helper-buttons",
                Button {
                    label: "Cancel",
                    common: Common { id: Some("helper-keep".to_owned()), ..Common::default() },
                    onclick: move |_| keep.msg(HelpersMsg::CancelRemove),
                }
                Button {
                    label: "Remove",
                    role: ButtonRole::Destructive,
                    common: Common { id: Some("helper-remove-yes".to_owned()), ..Common::default() },
                    onclick: move |_| remove.msg(HelpersMsg::Remove(helper.clone())),
                }
            }
        }
    }
}
