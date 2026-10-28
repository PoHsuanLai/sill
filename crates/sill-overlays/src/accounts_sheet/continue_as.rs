//! The "Continue as" question: an app offers to continue as one of the person's accounts. It
//! stands like the consent alert, but there is nothing to allow: the account is picked and the
//! person presses Continue. The answer is the same `Allow` once that porter reads as the pick.

use dioxus::prelude::*;
use ds::components::controls::button::Button;
use ds::components::controls::button_model::{Answers, ButtonRole};
use ds_shell::accounts::model::{
    AccountChoice, AllowScope, ChoiceKey, ConsentAnswer, PickerChoice,
};
use ds_shell::prelude::AccountPicker;

use super::words::{continue_message, continue_title};

/// The column: headline, the line under it, the account pop-up when several fit, then Continue
/// (Return) and Cancel (Escape).
#[component]
pub(super) fn ContinueBody(
    #[props(into)] app: String,
    choices: Vec<AccountChoice>,
    chosen: Option<ChoiceKey>,
    on_choose: EventHandler<ChoiceKey>,
    on_answer: EventHandler<ConsentAnswer>,
) -> Element {
    let title = continue_title(&app);
    let message = continue_message(&choices);
    let picked = chosen
        .as_ref()
        .filter(|key| choices.iter().any(|choice| &choice.key == *key))
        .or_else(|| choices.first().map(|choice| &choice.key))
        .cloned();
    let go = {
        let picked = picked.clone();
        move |()| {
            if let Some(account) = picked.clone() {
                on_answer.call(ConsentAnswer::Allow {
                    account,
                    scope: AllowScope::Once,
                });
            }
        }
    };
    let go_key = go.clone();
    let several = choices.len() > 1;
    rsx! {
        div {
            class: "ds-alert",
            onkeydown: move |event| match event.key() {
                Key::Escape => {
                    event.stop_propagation();
                    event.prevent_default();
                    on_answer.call(ConsentAnswer::Dismiss);
                }
                Key::Enter => {
                    event.stop_propagation();
                    event.prevent_default();
                    go_key(());
                }
                _ => {}
            },
            div { class: "ds-alert-title", "{title}" }
            div { class: "ds-alert-body", "{message}" }
            if several {
                div { class: "ds-acc-choice",
                    AccountPicker {
                        accounts: choices,
                        chosen: picked,
                        on_pick: move |choice: PickerChoice| match choice {
                            PickerChoice::Account(key) => on_choose.call(key),
                            PickerChoice::Add => on_answer.call(ConsentAnswer::AddAccount),
                        },
                    }
                }
            }
            div { class: "ds-alert-footer", "data-layout": "stack",
                span { class: "ds-alert-slot",
                    Button {
                        label: "Continue".to_owned(),
                        answers: Answers::Return,
                        role: ButtonRole::Normal,
                        autofocus: true,
                        onclick: move |_| go(()),
                    }
                }
                span { class: "ds-alert-slot",
                    Button {
                        label: "Cancel".to_owned(),
                        answers: Answers::Escape,
                        role: ButtonRole::Normal,
                        autofocus: false,
                        onclick: move |_| on_answer.call(ConsentAnswer::Dismiss),
                    }
                }
            }
        }
    }
}
