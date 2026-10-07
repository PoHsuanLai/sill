//! The password field the lock screen and the polkit prompt share (design/30 section 2.10):
//! a `TextField { Secure }` with the caps-lock mark beside it, shaking as one when the password
//! was wrong. Markup: `div.ds-password-field[data-form][data-filled][data-pulse]` holding the
//! field, `.ds-password-caps` and the caller's trailing button; `a-shake-x` on the root while it
//! shakes. `Form::Pill` is the lock screen's flat glass pill, `Form::Boxed` the polkit sheet's
//! bezeled box.

use crate::lock::password::Password;
use crate::lock::vocab::CapsLock;
use dioxus::prelude::*;
use ds::base::vocab::Availability;
use ds::base::word::Word;
use ds::components::fields::text_field::TextField;
use ds::components::fields::text_field_focus::FieldFocus;
use ds::components::fields::text_field_model::{FieldBezel, FieldKind};
use ds::style::icon::Icon;
use ds::style::icon::render::{Glyph, IconSize};

/// How the field is drawn: `data-form`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Word)]
pub(crate) enum Form {
    /// The lock screen's pill of flat glass, its enter button inside it.
    #[default]
    Pill,
    /// The polkit sheet's boxed field.
    Boxed,
}

/// What Escape does in the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Escape {
    /// It empties the field (the lock screen has nothing to cancel).
    Clears,
    /// It leaves the field alone, so the sheet around it cancels.
    Passes,
}

/// The shared field. `trailing` is the caller's button after the caps mark; Enter calls
/// `onsubmit`.
#[component]
pub(crate) fn PasswordField(
    password: Password,
    availability: Availability,
    caps: CapsLock,
    form: Form,
    #[props(into)] placeholder: String,
    escape: Escape,
    onsubmit: EventHandler<()>,
    #[props(default)] trailing: Option<Element>,
) -> Element {
    let pulse = password.shake.attrs();
    let class = match &pulse {
        Some((anim, _)) => format!("ds-password-field {anim}"),
        None => "ds-password-field".to_owned(),
    };
    let alias = pulse.map(|(_, alias)| alias);
    let bezel = match form {
        Form::Pill => FieldBezel::Plain,
        Form::Boxed => FieldBezel::Bezeled,
    };
    rsx! {
        div {
            class,
            "data-form": form.slug(),
            "data-pulse": alias,
            "data-filled": password.filled().slug(),
            "aria-disabled": availability.aria_disabled(),
            for round in [password.key()] {
                TextField {
                    key: "{round}",
                    bezel,
                    kind: FieldKind::Secure,
                    label: "Password",
                    value: "",
                    placeholder: placeholder.clone(),
                    availability,
                    focus: FieldFocus::OnMount,
                    oninput: move |next: String| password.input(next),
                    onkey: move |event: KeyboardEvent| match (event.key(), escape) {
                        (Key::Enter, _) => onsubmit.call(()),
                        (Key::Escape, Escape::Clears) => {
                            event.prevent_default();
                            password.clear();
                        }
                        _ => {}
                    },
                }
            }
            if caps == CapsLock::On {
                span { class: "ds-password-caps", role: "img", "aria-label": "Caps Lock is on",
                    Glyph { icon: Icon::CapsLock, size: IconSize::Compact }
                }
            }
            if let Some(trailing) = trailing {
                {trailing}
            }
        }
    }
}
