//! PolkitPrompt: a program asks for the person's password (design/20-SURFACES.md section 1.10;
//! design/04-COMPONENTS.md section 42). A narrow centred `Sheet` (it dims nothing), sliding in
//! from the top: the person's picture, a bold title, the program's message, "Details" as a
//! tooltip, the password in a boxed `Secret` field that shakes once when it was wrong, then Cancel
//! and Authenticate.

use crate::lock::password::use_password;
use crate::lock::password_field::{Escape, Form, PasswordField};
use crate::lock::picture::AT_POLKIT;
use crate::lock::vocab::{CapsLock, LockUser, PromptState};
use crate::user_picture::draw::drawn;
use dioxus::prelude::*;
use ds::base::vocab::Availability;
use ds::base::vocab::Shown;
use ds::components::content::text_runs::{TextLine, text};
use ds::components::controls::button::Button;
use ds::components::controls::button_model::Answers;
use ds::components::overlays::sheet::Sheet;
use ds::components::overlays::sheet_attach::Attach;
use ds::components::overlays::sheet_width::SheetWidth;
use ds::components::overlays::tooltip::Tooltip;
use ds::root::common::Common;

/// The title when the caller gives none.
const AUTHENTICATE: &str = "Authentication Required";

/// A password prompt for a privileged action. `action` is the program's message ("Authentication
/// is required to change the system's time"); `detail` (the action id, the program's path)
/// shows as a hover card on "Details". The password is a `Secret` field (no `value` prop, its
/// text never in the markup): `oninput` hears it, Enter or Authenticate hands it to `onsubmit`,
/// and Cancel, Escape or a click on the scrim call `oncancel`. `state` is the caller's, as for
/// [`crate::lock::prompt::LockPrompt`]: `Checking` closes the field and the buttons but Cancel, `Wrong` shakes
/// the field once and empties it, `LockedOut` says when it opens again. `shown` and `on_hidden`
/// are the sheet's own, for a host that unmaps its surface after the exit. `common` is the
/// sheet's: its `id` names the panel, for a host whose blur region resolves an element id.
#[component]
pub fn PolkitPrompt(
    #[props(into)] action: TextLine,
    #[props(default)] detail: Option<TextLine>,
    #[props(default)] title: Option<String>,
    user: LockUser,
    #[props(default)] state: PromptState,
    #[props(default)] caps: CapsLock,
    oninput: EventHandler<String>,
    onsubmit: EventHandler<String>,
    oncancel: EventHandler<()>,
    #[props(default)] shown: Option<Shown>,
    #[props(default)] on_hidden: Option<EventHandler<()>>,
    #[props(default)] common: Common,
) -> Element {
    let password = use_password(&state, oninput);
    let availability = state.availability();
    let go = move || {
        if availability == Availability::Enabled {
            password.submit(onsubmit);
        }
    };
    let out = match &state {
        PromptState::LockedOut { until } => Some(format!("Too many tries. Try again at {until}.")),
        _ => None,
    };
    rsx! {
        Sheet {
            label: "Authenticate",
            onclose: move |()| oncancel.call(()),
            shown,
            on_hidden,
            attach: Attach::Centre,
            width: SheetWidth::Narrow,
            common,
            div { class: "ds-polkit", "data-state": state.slug(),
                {drawn(user.picture, AT_POLKIT)}
                div { class: "ds-polkit-title", {title.unwrap_or_else(|| AUTHENTICATE.to_owned())} }
                div { class: "ds-polkit-action", {text(&action)} }
                if let Some(detail) = detail {
                    div { class: "ds-polkit-details",
                        Tooltip { text: detail.plain_text(),
                            span { class: "ds-polkit-details-word", "Details" }
                        }
                    }
                }
                div { class: "ds-polkit-user", "{user.name}" }
                PasswordField {
                    password,
                    availability,
                    caps,
                    form: Form::Boxed,
                    placeholder: "Password",
                    escape: Escape::Passes,
                    onsubmit: move |()| go(),
                }
                if let Some(line) = out {
                    div { class: "ds-polkit-hint", "{line}" }
                }
                div { class: "ds-polkit-actions",
                    Button { answers: Answers::Escape, label: "Cancel", onclick: move |_| oncancel.call(()) }
                    Button { answers: Answers::Return, label: "Authenticate", availability, onclick: move |_| go() }
                }
            }
        }
    }
}
