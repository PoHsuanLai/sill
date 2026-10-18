//! The quiet line under a card's fields when no model can see the screen: what cuad says, and a
//! button to the Settings page where the model is chosen.

use dioxus::prelude::*;
use ds::components::controls::button_model::Bezel;
use ds::prelude::*;
use sill_model::companion::{HelpersMsg, SightState};
use sill_services::CompanionService;
use sill_ui_kit::hooks::use_service;

use super::say::use_say;

/// Nothing while the screen can be seen (or is not known); otherwise the sentence and the way
/// to choose another model.
#[component]
pub(super) fn SightLine() -> Element {
    let companion = use_service::<CompanionService>();
    let say = use_say();
    let SightState::CannotSee(note) = companion.read().sight.clone() else {
        return rsx! {};
    };
    let sentence = note.as_str().to_owned();
    rsx! {
        div { class: "sill-helper-sight",
            p { class: "sill-helper-notice", role: "status", "{sentence}" }
            Button {
                label: "Choose a model",
                bezel: Bezel::Inline,
                common: Common { id: Some("helper-sight-model".to_owned()), ..Common::default() },
                onclick: move |_| say.msg(HelpersMsg::AddModel),
            }
        }
    }
}
