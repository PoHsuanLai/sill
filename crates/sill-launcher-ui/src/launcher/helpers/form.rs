//! The card's fields: its name, what it is for, the apps it may use and how far, and its model.

use std::collections::BTreeSet;

use dioxus::prelude::*;
use docket_core::helper::Ceiling;
use ds::components::content::label::LabelRole;
use ds::components::controls::button_model::Bezel;
use ds::components::controls::checkbox::Checkbox;
use ds::components::fields::field_row::{FieldGroup, FieldRow};
use ds::prelude::*;
use prov::AppName;
use sill_model::companion::{
    CEILINGS, Draft, HelpersMsg, Reach, app_label, ceiling_label, model_hint, model_line,
};

use super::say::{Say, use_say};

/// The field group of a card being filled in. `name` and `role` hold what is typed until it is
/// committed; `apps` are the apps that can be ticked.
#[component]
pub(super) fn Fields(
    draft: Draft,
    apps: BTreeSet<AppName>,
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
