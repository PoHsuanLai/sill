//! The lock surface's content inside its `Ds` root: quire's stage, the clock, the prompt on the
//! prompt's output, and the unlock fade (the module doc of `screen.rs` has the rules).

use dioxus::prelude::*;
use ds::{
    Anim, CapsLock, ImageSource, LockPrompt, LockScreen, TextLine, TimerPhase, use_motion_timer,
};
use shell_host::OutputInfo;
use sill_model::auth::{Auth, AuthCommand, AuthFault, AuthPhase, PamNote, Secret};
use sill_model::ids::OutputName;

use super::backdrop::backdrop_for;
use super::model::{clock_lines, lock_user, prompt_state};
use crate::hooks::{use_service, use_services, use_settings};
use crate::links::{LockLink, PromptOutput};
use sill_services::{AuthService, ClockService, WallpaperService};

/// Where shell-host's session lock is, as the surface needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum HostLock {
    /// Requested or locked: the surface fades and unlocks on `Accepted`.
    Holding,
    /// The host is already unlocking (the daemon's backstop) or the lock ended: no fade.
    Ending,
}

/// Whether the prompt is on this surface's output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Here {
    Yes,
    No,
}

/// The `data-leaving` word: the surface fades once the password was right.
fn leaving(phase: &AuthPhase, host: HostLock) -> Option<&'static str> {
    match (phase, host) {
        (AuthPhase::Accepted, HostLock::Holding) => Some("yes"),
        _ => None,
    }
}

/// The line under the field: the last PAM message, else a word for a check that could not run.
fn hint_of(auth: &Auth) -> Option<TextLine> {
    let note = auth.notes.last().map(|note| match note {
        PamNote::Info(text) | PamNote::Error(text) => TextLine::from(text.as_str()),
    });
    note.or(match auth.phase {
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
    scheme: ds::Scheme,
    host: HostLock,
    caps: CapsLock,
    on_unlock: Option<Callback<()>>,
) -> Element {
    let services = use_services();
    let settings = use_settings()();
    let wallpapers = use_service::<WallpaperService>()();
    let auth_signal = use_service::<AuthService>();
    let auth = auth_signal();
    // The bar clock's text changes each minute: reading it re-renders the lock clock with it.
    let _minute = use_service::<ClockService>()();
    let link = use_context::<LockLink>();
    let prompt_at = shell_host::use_watch(link.subscribe())();
    let here = match prompt_at {
        PromptOutput::On(id) if id == output.id => Here::Yes,
        PromptOutput::On(_) | PromptOutput::Nowhere => Here::No,
    };
    use_unlock_fade(auth_signal, host, here, on_unlock);

    let format = settings.session.lock_clock;
    let now = jiff::Zoned::now();
    let lines = clock_lines(&now, format);
    let wallpaper = output
        .name
        .clone()
        .map(OutputName)
        .and_then(|name| backdrop_for(&wallpapers, &name, scheme))
        .filter(|path| path.is_file())
        .and_then(|path| ImageSource::file(&path).ok());
    let prompt = match here {
        Here::Yes => {
            let control = services.auth.control.clone();
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
        }
        Here::No => rsx! {},
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
                    ds::LockClock { time: lines.time, date: lines.date }
                },
                prompt,
            }
        }
    }
}

/// On `Accepted` while the host holds the lock, the prompt's surface times the fade and then
/// asks shell-host to unlock; the others only fade.
fn use_unlock_fade(
    auth: ReadSignal<Auth>,
    host: HostLock,
    here: Here,
    on_unlock: Option<Callback<()>>,
) {
    // quire's accepted beat (the picture's accept) settles before the unlock.
    let timer = use_motion_timer(Anim::PictureAccept);
    use_effect(use_reactive!(|host, here, on_unlock| {
        let accepted = auth.read().phase == AuthPhase::Accepted;
        match (accepted, host, here, on_unlock) {
            (true, HostLock::Holding, Here::Yes, Some(unlock)) => {
                if timer.phase() == TimerPhase::Idle {
                    timer.start(EventHandler::new(move |()| unlock.call(())));
                }
            }
            (false, _, _, _) => timer.cancel(),
            _ => {}
        }
    }));
}
