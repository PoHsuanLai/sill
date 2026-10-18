//! Automatic, under a helper's list of models: the button that starts it, and while the person is
//! choosing, the models to tick and the two answers. Nothing is ticked for them.

use dioxus::prelude::*;
use ds::components::content::label::{LabelRole, LabelStyle};
use ds::components::controls::button_model::Bezel;
use ds::components::controls::checkbox::Checkbox;
use ds::prelude::*;
use sill_model::companion::{AUTOMATIC_HELP, Automatic, HelpersMsg, ModelGroup, ModelOffer};

use super::say::{Say, use_say};

/// The part of the model list that is Automatic's.
#[component]
pub(super) fn AutomaticPool(groups: Vec<ModelGroup>, automatic: Automatic) -> Element {
    let say = use_say();
    let Automatic::Choosing(_) = &automatic else {
        let start = say;
        return rsx! {
            Button {
                label: "Automatic\u{2026}",
                bezel: Bezel::Inline,
                common: Common { id: Some("helper-automatic".to_owned()), ..Common::default() },
                onclick: move |_| start.msg(HelpersMsg::StartAutomatic),
            }
        };
    };
    let (confirm, cancel) = (say.clone(), say.clone());
    rsx! {
        div { class: "sill-helper-pool",
            Label { text: AUTOMATIC_HELP, role: LabelRole::Secondary, style: LabelStyle::Footnote }
            for group in groups.iter() {
                Label { key: "{group.heading}", text: group.heading.clone(), style: LabelStyle::Caption }
                for (at, offer) in group.offers.iter().enumerate() {
                    {pooled_row(&group.heading, at, offer, &automatic, &say)}
                }
            }
            div { class: "sill-helper-buttons",
                Button {
                    label: "Cancel",
                    common: Common { id: Some("helper-automatic-cancel".to_owned()), ..Common::default() },
                    onclick: move |_| cancel.msg(HelpersMsg::CancelAutomatic),
                }
                Button {
                    label: "Use Automatic",
                    common: Common { id: Some("helper-automatic-confirm".to_owned()), ..Common::default() },
                    onclick: move |_| confirm.msg(HelpersMsg::ConfirmAutomatic),
                }
            }
        }
    }
}

/// One model as a tick box.
fn pooled_row(
    heading: &str,
    at: usize,
    offer: &ModelOffer,
    automatic: &Automatic,
    say: &Say,
) -> Element {
    let Some(model) = offer.model().cloned() else {
        return rsx! {};
    };
    let value = if automatic.includes(offer) {
        Check::On
    } else {
        Check::Off
    };
    let say = say.clone();
    rsx! {
        Checkbox {
            key: "{heading}-{at}",
            label: offer.label.clone(),
            value,
            onchange: move |_| say.msg(HelpersMsg::TogglePooled(model.clone())),
        }
        if let Some(facts) = offer.facts() {
            Label { text: facts, role: LabelRole::Secondary, style: LabelStyle::Footnote }
        }
    }
}
