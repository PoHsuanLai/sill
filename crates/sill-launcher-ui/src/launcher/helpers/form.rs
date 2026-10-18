//! The card's fields: its name, what it is for, the apps it may use and how far, and its model.

use std::collections::BTreeSet;

use dioxus::prelude::*;
use docket_core::helper::Ceiling;
use ds::components::content::label::{LabelRole, LabelStyle};
use ds::components::controls::button_model::Bezel;
use ds::components::controls::checkbox::Checkbox;
use ds::components::fields::field_row::{FieldGroup, FieldRow};
use ds::prelude::*;
use prov::{AppName, HelperId};
use sill_model::companion::{
    ADD_MODEL, Automatic, CEILINGS, Draft, HelpersMsg, LOOKING_FOR_MODELS, ModelGroup, ModelList,
    ModelOffer, NO_MODELS, Reach, app_label, ceiling_label, model_hint, model_line,
};

use super::automatic::AutomaticPool;
use super::say::{Say, use_say};

/// The field group of a card being filled in. `name` and `role` hold what is typed until it is
/// committed; `apps` are the apps that can be ticked; `models` is the list under the model.
#[component]
pub(super) fn Fields(
    draft: Draft,
    apps: BTreeSet<AppName>,
    models: ModelList,
    automatic: Automatic,
    helper: Option<HelperId>,
    name: Signal<String>,
    role: Signal<String>,
) -> Element {
    let say = use_say();
    let (mut name, mut role) = (name, role);
    let (ticked, ceiling) = match &draft.reach {
        Reach::Chosen { apps, ceiling } => (apps.clone(), *ceiling),
        Reach::Unreadable => (BTreeSet::new(), Ceiling::Look),
    };
    let offered: BTreeSet<AppName> = apps.union(&ticked).cloned().collect();
    let uses_help = match &draft.reach {
        Reach::Chosen { .. } if offered.is_empty() => {
            Some(TextLine::from("No app is installed that a helper can use."))
        }
        Reach::Chosen { .. } => None,
        Reach::Unreadable => Some(TextLine::from("Choose the apps again to save this card.")),
    };
    let choices: Vec<Choice<Ceiling>> = CEILINGS
        .iter()
        .map(|ceiling| Choice::new(*ceiling, ceiling_label(*ceiling)))
        .collect();
    let (name_say, role_say, far_say, pick_say) =
        (say.clone(), say.clone(), say.clone(), say.clone());
    rsx! {
        FieldGroup {
            FieldRow { label: "Name",
                TextField {
                    label: "Name",
                    value: name(),
                    placeholder: "Research",
                    oninput: move |next: String| name.set(next),
                    onchange: move |text: String| name_say.msg(HelpersMsg::EditName(text)),
                }
            }
            FieldRow {
                label: "For",
                help: Some(TextLine::from("What it does for you, in your own words.")),
                TextField {
                    label: "What it is for",
                    kind: FieldKind::Multiline,
                    value: role(),
                    oninput: move |next: String| role.set(next),
                    onchange: move |text: String| role_say.msg(HelpersMsg::EditRole(text)),
                }
            }
            FieldRow { label: "Uses", help: uses_help,
                div { class: "sill-helper-apps",
                    for app in offered.iter() {
                        {app_row(app, &ticked, &say)}
                    }
                }
            }
            FieldRow { label: "How far",
                RadioGroup {
                    label: "How far it may go",
                    choices,
                    value: ceiling,
                    onchange: move |chosen: Ceiling| far_say.msg(HelpersMsg::SetCeiling(chosen)),
                }
            }
            FieldRow {
                label: "Model",
                help: model_hint(&draft.model).map(TextLine::from),
                Label { text: model_line(&draft.model), role: LabelRole::Secondary }
                Button {
                    label: "Choose\u{2026}",
                    bezel: Bezel::Inline,
                    common: Common { id: Some("helper-model".to_owned()), ..Common::default() },
                    onclick: move |_| pick_say.msg(HelpersMsg::PickModel),
                }
                {model_list(&models, &automatic, helper.as_ref(), &say)}
            }
        }
    }
}

/// One app as a tick box.
fn app_row(app: &AppName, ticked: &BTreeSet<AppName>, say: &Say) -> Element {
    let value = if ticked.contains(app) {
        Check::On
    } else {
        Check::Off
    };
    let (say, toggled) = (say.clone(), app.clone());
    rsx! {
        Checkbox {
            key: "{app}",
            label: app_label(app),
            value,
            onchange: move |_| say.msg(HelpersMsg::ToggleApp(toggled.clone())),
        }
    }
}

/// The models to choose from, under the model's line: what is being read, or the models grouped by
/// where they run, with "Add a model in Settings" last (the only entry when there are none).
fn model_list(
    models: &ModelList,
    automatic: &Automatic,
    helper: Option<&HelperId>,
    say: &Say,
) -> Element {
    match models {
        ModelList::Closed => rsx! {},
        ModelList::Reading => rsx! {
            Label { text: LOOKING_FOR_MODELS, role: LabelRole::Secondary }
        },
        ModelList::Offered(groups) => {
            let mut first = 0;
            let mut shown = Vec::new();
            for group in groups {
                shown.push(group_rows(group, first, say));
                first += group.offers.len();
            }
            let add = say.clone();
            let pool = match helper {
                Some(_) => rsx! {
                    AutomaticPool { groups: groups.clone(), automatic: automatic.clone() }
                },
                None => rsx! {},
            };
            rsx! {
                div { class: "sill-helper-models",
                    if groups.is_empty() {
                        Label { text: NO_MODELS, role: LabelRole::Secondary }
                    }
                    for rows in shown {
                        {rows}
                    }
                    {pool}
                    Button {
                        label: ADD_MODEL,
                        bezel: Bezel::Inline,
                        common: Common { id: Some("helper-model-add".to_owned()), ..Common::default() },
                        onclick: move |_| add.msg(HelpersMsg::AddModel),
                    }
                }
            }
        }
    }
}

/// One heading and its models; `first` numbers the buttons across groups.
fn group_rows(group: &ModelGroup, first: usize, say: &Say) -> Element {
    rsx! {
        Label { text: group.heading.clone(), style: LabelStyle::Caption }
        for (at, offer) in group.offers.iter().enumerate() {
            {offer_row(first + at, offer, say)}
        }
    }
}

/// One model as a button that chooses it.
fn offer_row(index: usize, offer: &ModelOffer, say: &Say) -> Element {
    let (say, choice) = (say.clone(), offer.choice.clone());
    rsx! {
        Button {
            key: "{index}",
            label: offer.label.clone(),
            bezel: Bezel::Inline,
            common: Common { id: Some(format!("helper-model-{index}")), ..Common::default() },
            onclick: move |_| say.msg(HelpersMsg::ChooseModel(choice.clone())),
        }
        if let Some(facts) = offer.facts() {
            Label { text: facts, role: LabelRole::Secondary, style: LabelStyle::Footnote }
        }
    }
}
