//! The app a helper runs: what the card says about it, where the sign-in stands, and the buttons
//! that sign in and out. The words and the choice of buttons are the model's
//! (`sill_model::companion`); this draws them.

use companion_wire::HelperCardView;
use dioxus::prelude::*;
use ds::components::content::label::{LabelRole, LabelStyle};
use ds::components::controls::button_model::Bezel;
use ds::prelude::*;
use sill_model::companion::{
    HelpersMsg, SignInButton, kind_line, login_line, offers_sign_out, sign_in_buttons,
};

use super::handover::HandoverRow;
use super::say::{Say, use_say};

/// "Uses Claude Code", whether the person is signed in, and the buttons; nothing for a helper
/// that plans for itself.
#[component]
pub(super) fn AppLine(view: HelperCardView) -> Element {
    let say = use_say();
    let Some(kind) = kind_line(&view) else {
        return rsx! {};
    };
    let helper = view.line.id.clone();
    let sign_out = if offers_sign_out(&view) {
        let say = say.clone();
        rsx! {
            Button {
                label: "Sign Out",
                bezel: Bezel::Inline,
                common: Common { id: Some("helper-sign-out".to_owned()), ..Common::default() },
                onclick: move |_| say.msg(HelpersMsg::SignOut(helper.clone())),
            }
        }
    } else {
        rsx! {}
    };
    rsx! {
        div { class: "sill-helper-app",
            Label { text: kind, role: LabelRole::Secondary }
            if let Some(login) = login_line(&view) {
                Label { text: login, style: LabelStyle::Caption }
            }
            div { class: "sill-helper-buttons",
                {sign_in_row(&view, "helper-sign-in", &say)}
                {sign_out}
            }
            HandoverRow { view }
        }
    }
}

/// The buttons that sign in; `prefix` numbers their ids.
pub(super) fn sign_in_row(view: &HelperCardView, prefix: &str, say: &Say) -> Element {
    let buttons = sign_in_buttons(view);
    rsx! {
        for (at, button) in buttons.into_iter().enumerate() {
            {sign_in_button(format!("{prefix}-{at}"), button, view, say)}
        }
    }
}

fn sign_in_button(id: String, button: SignInButton, view: &HelperCardView, say: &Say) -> Element {
    let (say, helper) = (say.clone(), view.line.id.clone());
    let SignInButton { label, way } = button;
    rsx! {
        Button {
            key: "{id}",
            label,
            common: Common { id: Some(id), ..Common::default() },
            onclick: move |_| say.msg(HelpersMsg::SignIn(helper.clone(), way.clone())),
        }
    }
}
