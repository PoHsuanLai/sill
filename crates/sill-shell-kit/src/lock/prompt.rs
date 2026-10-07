//! LockPrompt: who is asked, and the password field (design/20-SURFACES.md section 1.9;
//! design/04-COMPONENTS.md section 42). The person's picture, the name, a pill field of flat
//! white glass with an enter arrow inside it, a caps-lock mark, and a hint line under it.

use crate::lock::password::{Filled, use_password};
use crate::lock::password_field::{Escape, Form, PasswordField};
use crate::lock::picture::AT_LOCK;
use crate::lock::vocab::{CapsLock, LockLook, LockUser, PromptState};
use crate::user_picture::draw::drawn;
use dioxus::prelude::*;
use ds::base::vocab::Availability;
use ds::base::word::Word;
use ds::components::content::text_runs::{TextLine, text};
use ds::components::controls::progress::model::{Progress, ProgressStyle};
use ds::components::controls::progress::view::ProgressIndicator;
use ds::motion::detail::{
    operation::Operation, touch::Touch, use_detail::use_detail, use_operation::use_operation,
};
use ds::root::common::Common;
use ds::style::icon::Icon;
use ds::style::icon::render::{Glyph, IconSize};
use ds::style::tokens::control_size::ControlSize;

/// The placeholder when the caller gives none.
const ENTER_PASSWORD: &str = "Enter Password";

/// The lock screen's prompt. The password is a `Secure` field: its text never reaches the
/// markup, and there is no `value` prop. `oninput` hears every change (an empty string when the
/// prompt empties the field itself); Enter or the arrow hands the text to `onsubmit`; Escape
/// empties the field. `state` is the caller's: `Checking` spins the arrow and closes the field,
/// `Wrong` shakes the field once and empties it when the shake settles, `LockedOut` closes the
/// field and says when it opens. `caps` marks caps lock; `hint` is the line under the field
/// ("Touch the key or enter your password"). The field takes the keyboard as it mounts.
///
/// `user.picture` is drawn at 64: a face, a photo, or an emoji playing its own animation once as
/// the prompt appears.
#[component]
pub fn LockPrompt(
    user: LockUser,
    #[props(default)] state: PromptState,
    #[props(default)] caps: CapsLock,
    #[props(default)] look: LockLook,
    #[props(default)] placeholder: Option<String>,
    #[props(default)] hint: Option<TextLine>,
    oninput: EventHandler<String>,
    onsubmit: EventHandler<String>,
    #[props(default)] common: Common,
) -> Element {
    let password = use_password(&state, oninput);
    let line = hint_line(&state, hint);
    // Checking is an operation the prompt's own state starts.
    let operation = use_operation(use_detail(state.clone(), Touch::Remote).cue());
    let availability = state.availability();
    let go = EventHandler::new(move |()| {
        if availability == Availability::Enabled {
            password.submit(onsubmit);
        }
    });
    let class = common.class("ds-lock-prompt");
    let data = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-look": look.slug(),
            "data-state": state.slug(),
            "aria-label": common.aria_label.clone(),
            onmounted: move |event| common.mounted(event),
            ..data,
            {drawn(user.picture, AT_LOCK)}
            div { class: "ds-lock-name", "{user.name}" }
            PasswordField {
                password,
                availability,
                caps,
                form: Form::Pill,
                placeholder: placeholder.unwrap_or_else(|| ENTER_PASSWORD.to_owned()),
                escape: Escape::Clears,
                onsubmit: go,
                trailing: Some(go_button(&state, password.filled(), go, operation)),
            }
            if let Some(line) = line {
                div { class: "ds-lock-hint", {text(&line)} }
            }
        }
    }
}

/// The line under the field: the lock-out's time while locked out, else the caller's hint.
fn hint_line(state: &PromptState, hint: Option<TextLine>) -> Option<TextLine> {
    match state {
        PromptState::LockedOut { until } => Some(TextLine::from(format!("Try again at {until}"))),
        _ => hint,
    }
}

/// The enter arrow inside the pill: a spinner while the password is tried.
fn go_button(
    state: &PromptState,
    filled: Filled,
    onpress: EventHandler<()>,
    operation: Operation,
) -> Element {
    let availability = state.availability();
    let checking = *state == PromptState::Checking;
    rsx! {
        button {
            r#type: "button",
            class: "ds-lock-go",
            "aria-label": "Unlock",
            "data-filled": filled.slug(),
            "aria-disabled": availability.aria_disabled(),
            "aria-busy": checking.then_some("true"),
            onclick: move |_| onpress.call(()),
            if checking {
                span { class: "ds-lock-busy",
                    ProgressIndicator {
                        style: ProgressStyle::Spinner,
                        progress: Progress::Unknown(operation),
                        size: ControlSize::Small,
                    }
                }
            } else {
                Glyph { icon: Icon::ArrowRight, size: IconSize::Small }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::hint_line;
    use crate::lock::vocab::PromptState;
    use ds::components::content::text_runs::TextLine;

    #[test]
    fn a_lock_out_speaks_over_the_hint() {
        let hint = || Some(TextLine::from("Enter your password"));
        let out = PromptState::LockedOut {
            until: "9:52".into(),
        };
        assert_eq!(
            hint_line(&out, hint()).map(|line| line.plain_text()),
            Some("Try again at 9:52".to_owned())
        );
        assert_eq!(
            hint_line(&PromptState::Wrong, hint()).map(|line| line.plain_text()),
            Some("Enter your password".to_owned())
        );
        assert_eq!(hint_line(&PromptState::Idle, None), None);
    }
}
