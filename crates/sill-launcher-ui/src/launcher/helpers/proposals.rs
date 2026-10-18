//! The routines a helper suggests, in its Routines section: what each would do and when, the
//! suggester's reason as a quotation (it is theirs, not ours), and Accept or Not Now. Nothing is
//! made until the person accepts.

use companion_wire::RoutineProposal;
use dioxus::prelude::*;
use ds::components::content::label::{LabelRole, LabelStyle};
use ds::prelude::*;
use sill_model::companion::{
    HelpersMsg, MISSED_SKIPPED, PROPOSAL_ACCEPT, PROPOSAL_DECLINE, PROPOSAL_TITLE,
    PROPOSAL_WHY_LEAD, RoutinesMsg, proposal_what, proposal_when, proposal_why,
};

use super::say::Say;

/// One suggestion as a panel the person answers.
pub(super) fn proposal_panel(proposal: &RoutineProposal, say: &Say) -> Element {
    let id = proposal.id.clone();
    let (accept_id, decline_id) = (id.clone(), id.clone());
    let (accept, decline) = (say.clone(), say.clone());
    rsx! {
        div { key: "{id}", class: "sill-helper-ask", role: "group",
            Label { text: PROPOSAL_TITLE, style: LabelStyle::Headline }
            Label { text: proposal_what(proposal).to_owned() }
            Label { text: proposal_when(proposal), role: LabelRole::Secondary, style: LabelStyle::Footnote }
            Label { text: MISSED_SKIPPED, role: LabelRole::Secondary, style: LabelStyle::Footnote }
            if let Some(why) = proposal_why(proposal) {
                Label { text: PROPOSAL_WHY_LEAD, role: LabelRole::Secondary, style: LabelStyle::Footnote }
                Label { text: why, role: LabelRole::Secondary, style: LabelStyle::Footnote }
            }
            div { class: "sill-helper-buttons",
                Button {
                    label: PROPOSAL_DECLINE,
                    common: Common { id: Some(format!("helper-proposal-decline-{id}")), ..Common::default() },
                    onclick: move |_| decline.msg(answer(RoutinesMsg::DeclineProposal(decline_id.clone()))),
                }
                Button {
                    label: PROPOSAL_ACCEPT,
                    common: Common { id: Some(format!("helper-proposal-accept-{id}")), ..Common::default() },
                    onclick: move |_| accept.msg(answer(RoutinesMsg::AcceptProposal(accept_id.clone()))),
                }
            }
        }
    }
}

fn answer(msg: RoutinesMsg) -> HelpersMsg {
    HelpersMsg::Routines(msg)
}
