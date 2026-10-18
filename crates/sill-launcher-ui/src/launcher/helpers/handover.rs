//! Where the helper's work is: a button that opens it in a terminal, or, while it is in one,
//! the words "In Terminal" and a button that brings it back. What is offered is the model's
//! (`sill_model::companion::handover_of`); this draws it.

use companion_wire::HelperCardView;
use dioxus::prelude::*;
use ds::components::content::label::LabelRole;
use ds::components::controls::button_model::Bezel;
use ds::prelude::*;
use sill_model::companion::{
    BRING_BACK, HandoverShown, HelpersMsg, IN_TERMINAL, OPEN_IN_TERMINAL, handover_of,
};

use super::say::use_say;

/// The terminal buttons of a helper that runs an app; nothing when its work cannot move.
#[component]
pub(super) fn HandoverRow(view: HelperCardView) -> Element {
    let say = use_say();
    let helper = view.line.id.clone();
    match handover_of(&view) {
        HandoverShown::Nothing => rsx! {},
        HandoverShown::OffersTerminal => rsx! {
            div { class: "sill-helper-buttons",
                Button {
                    label: OPEN_IN_TERMINAL,
                    bezel: Bezel::Inline,
                    common: Common { id: Some("helper-open-terminal".to_owned()), ..Common::default() },
                    onclick: move |_| say.msg(HelpersMsg::OpenInTerminal(helper.clone())),
                }
            }
        },
        HandoverShown::InTerminal => rsx! {
            div { class: "sill-helper-buttons",
                Label { text: IN_TERMINAL, role: LabelRole::Secondary }
                Button {
                    label: BRING_BACK,
                    bezel: Bezel::Inline,
                    common: Common { id: Some("helper-bring-back".to_owned()), ..Common::default() },
                    onclick: move |_| say.msg(HelpersMsg::BringBack(helper.clone())),
                }
            }
        },
    }
}
