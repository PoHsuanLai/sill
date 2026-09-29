//! The lock surface's root.
//!
//! It draws `Ds { material: Window, extent: Viewport }` holding `ds::LockScreen { wallpaper, clock,
//! prompt }`: the wallpaper is [`super::backdrop_for`] over `use_service::<WallpaperService>()` in
//! the scheme `use_appearance()` resolves (`ImageSource::file`; `None` lets quire paint its veil
//! alone); the clock is `ds::LockClock` from [`super::clock_lines`] over the minute
//! (`use_service::<ClockService>()` wakes it) and `session.lock_clock`; the prompt, only on the
//! output the [`super::LockLink`] names, is `ds::LockPrompt` with [`super::lock_user`] over
//! `use_service::<AuthService>().account`, [`super::prompt_state`] over
//! `use_service::<AuthService>().phase`, the Caps Lock mark from shell-host's `use_modifiers()`
//! (published on keyboard focus, so a Caps Lock toggled before the lock shows from the first frame;
//! [`super::view::caps_of`]), and `onsubmit` sending `AuthCommand::Submit(Secret::new(text))` on
//! the auth service's control (the text is moved into the `Secret` there, never stored or logged).
//! On `Accepted` it fades and, on the prompt's output, calls `shell_host::use_lock().unlock()` when
//! the fade settles; `shell_host::use_lock_phase()` saying `Unlocking` first (the daemon's
//! backstop) skips the fade. A click on a lock surface moves the prompt there (macOS: the prompt
//! follows the display you click), as the keyboard's focus does (the daemon). Style nothing under
//! `.ds-lock*`: the fade is sill's own wrapper's.
//!
//! Outside a lock document (a test's SSR, a harness) there is no `LockHandle`: the stage draws
//! the same and the unlock is the daemon's backstop alone.

use dioxus::prelude::*;
use ds::{BlurState, CapsLock, Ds, Material, RootExtent};
use shell_host::{LockHandle, LockPhase, OutputInfo, use_lock, use_lock_phase, use_modifiers};

use super::body::{HostLock, LockBody};
use super::view::caps_of;
use crate::hooks::{use_appearance, use_ds_scale};

/// The lock screen's stylesheet (lint-clean under `Profile::Strict`, `tests/lock_lint.rs`).
pub(crate) const LOCK_CSS: &str = include_str!("../../style/lock.css");

/// Whether this document belongs to a session lock (fixed for a document's life).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hosted {
    Yes,
    No,
}

pub(super) fn lock_screen(output: OutputInfo) -> Element {
    let hosted =
        use_hook(|| try_consume_context::<LockHandle>().map_or(Hosted::No, |_| Hosted::Yes));
    match hosted {
        Hosted::Yes => rsx! {
            HostedStage { output }
        },
        Hosted::No => rsx! {
            Stage { output, host: HostLock::Holding, caps: CapsLock::Off, on_unlock: None }
        },
    }
}

/// The stage inside a session lock's document: it follows the lock's phase and unlocks it.
#[component]
fn HostedStage(output: OutputInfo) -> Element {
    let handle = use_lock();
    let host = match use_lock_phase()() {
        LockPhase::Requested | LockPhase::Locked => HostLock::Holding,
        LockPhase::Unlocking | LockPhase::Unlocked | LockPhase::Finished => HostLock::Ending,
    };
    let caps = caps_of(use_modifiers()().caps_lock);
    let on_unlock = use_callback(move |()| handle.unlock());
    rsx! {
        Stage { output, host, caps, on_unlock: Some(on_unlock) }
    }
}

/// The `Ds` root; everything that needs quire's environment is [`LockBody`].
#[component]
fn Stage(
    output: OutputInfo,
    host: HostLock,
    caps: CapsLock,
    on_unlock: Option<Callback<()>>,
) -> Element {
    let ds_scale = use_ds_scale();
    let environment = use_appearance()();
    let appearance = environment.settings.appearance.appearance();
    let scheme = ds::resolve(
        appearance,
        ds::SpaceLook::default().theme,
        environment.system,
    )
    .scheme;
    rsx! {
        Ds {
            scale: ds_scale,
            appearance,
            system: environment.system,
            material: Material::Window,
            extent: RootExtent::Viewport,
            blur: BlurState::Unavailable,
            style { {LOCK_CSS} }
            LockBody { output, scheme, host, caps, on_unlock }
        }
    }
}
