//! The lock surface's content inside its `Ds` root: quire's stage, the clock, the prompt on the
//! prompt's output, and the unlock fade (the module doc of `screen.rs` has the rules).

use dioxus::prelude::*;
use ds::prelude::*;
use shell_host::OutputInfo;
use sill_model::auth::{Auth, AuthCommand, AuthFault, AuthPhase, PamNote, Secret};
use sill_model::ids::OutputName;
use sill_shell_kit::lock::vocab::CapsLock;
use sill_shell_kit::prelude::*;

use super::backdrop::backdrop_for;
use super::model::{clock_lines, lock_user, prompt_state};
use sill_services::{AuthService, ClockService, WallpaperService};
use sill_ui_kit::hooks::{use_hours, use_service, use_services};
use sill_ui_kit::links::{LockLink, PromptOutput};

/// Where shell-host's session lock is, as the surface needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum HostLock {
    /// Requested or locked: the surface fades and unlocks on `Accepted`.
    Holding,
    /// The host is already unlocking (the daemon's backstop) or the lock ended: no fade.
    Ending,
}

/// The `data-leaving` word: the surface fades once the password was right.
fn leaving(phase: &AuthPhase, host: HostLock) -> Option<&'static str> {
    match (phase, host) {
        (AuthPhase::Accepted, HostLock::Holding) => Some("yes"),
        _ => None,
    }
}

/// What the line under the field says when the password was right but has expired: the login
/// screen asks for a new one, which the lock screen cannot.
const EXPIRED_HINT: &str = "Your password has expired. Sign in again to choose a new one.";

/// The line under the field: the last PAM message, else a word for a check that could not run.
fn hint_of(auth: &Auth) -> Option<TextLine> {
    let note = auth.notes.last().map(|note| match note {
        PamNote::Info(text) | PamNote::Error(text) => TextLine::from(text.as_str()),
    });
    note.or_else(|| match auth.phase {
        AuthPhase::Expired => Some(TextLine::from(EXPIRED_HINT)),
        AuthPhase::Unavailable(AuthFault::TimedOut) => {
            Some(TextLine::from("The check took too long"))
        }
        AuthPhase::Unavailable(_) => {
            Some(TextLine::from("The password can't be checked right now"))
        }
        _ => None,
    })
}

#[component]
pub(super) fn LockBody(
    output: OutputInfo,
    scheme: Scheme,
    host: HostLock,
    caps: CapsLock,
    on_unlock: Option<Callback<()>>,
) -> Element {
    let services = use_services();
    let hours = use_hours();
    let wallpapers = use_service::<WallpaperService>()();
    let auth_signal = use_service::<AuthService>();
    let auth = auth_signal();
    // The bar clock's text changes each minute: reading it re-renders the lock clock with it.
    let _minute = use_service::<ClockService>()();
    let link = use_context::<LockLink>();
    let prompt_at = shell_host::use_watch(link.subscribe())();
    // Whether the prompt is on this surface's output.
    let here = matches!(prompt_at, PromptOutput::On(id) if id == output.id);
    use_unlock_fade(auth_signal, host, here, on_unlock);

    let now = jiff::Zoned::now();
    let lines = clock_lines(&now, hours);
    let wallpaper = output
        .name
        .clone()
        .map(OutputName)
        .and_then(|name| backdrop_for(&wallpapers, &name, scheme))
        .filter(|path| path.is_file())
        .and_then(|path| ImageSource::file(&path).ok());
    let prompt = if here {
        let control = services.auth.control;
        rsx! {
            LockPrompt {
                user: lock_user(&auth.account),
                state: prompt_state(&auth.phase),
                caps,
                hint: hint_of(&auth),
                oninput: move |_: String| {},
                onsubmit: move |text: String| {
                    let _ = control.send(AuthCommand::Submit(Secret::new(text)));
                },
            }
        }
    } else {
        rsx! {}
    };
    let id = output.id;
    rsx! {
        div {
            class: "sill-lock",
            "data-leaving": leaving(&auth.phase, host),
            onmousedown: move |_| link.set(PromptOutput::On(id)),
            LockScreen {
                wallpaper,
                clock: rsx! {
                    LockClock { time: lines.time, date: lines.date }
                },
                prompt,
            }
        }
    }
}

/// On `Accepted` while the host holds the lock, asks shell-host to unlock at once (quire's
/// picture accept beat is gone); the others do nothing.
fn use_unlock_fade(
    auth: ReadSignal<Auth>,
    host: HostLock,
    here: bool,
    on_unlock: Option<Callback<()>>,
) {
    // Whether the unlock has been asked for this acceptance: not again until the prompt leaves
    // `Accepted`.
    let mut asked = use_hook(|| CopyValue::new(false));
    use_effect(use_reactive!(|host, here, on_unlock| {
        let accepted = auth.read().phase == AuthPhase::Accepted;
        match (accepted, host, here, on_unlock) {
            (true, HostLock::Holding, true, Some(unlock)) => {
                if !*asked.peek() {
                    asked.set(true);
                    unlock.call(());
                }
            }
            (false, _, _, _) => asked.set(false),
            _ => {}
        }
    }));
}
