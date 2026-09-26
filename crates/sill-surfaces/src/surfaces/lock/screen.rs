//! The lock surface's root (lane a fills it; it draws nothing until then).
//!
//! What it draws once filled: `Ds { material: Window, extent: Viewport }` holding
//! `ds::LockScreen { wallpaper, clock, prompt }`: the wallpaper is [`super::backdrop_for`] over
//! `use_wallpapers()` in the scheme `use_appearance()` resolves (`ImageSource::file`; `None`
//! lets quire paint its veil alone); the clock is `ds::LockClock` from [`super::clock_lines`]
//! over the minute (`use_clock()` wakes it) and `session.lock_clock`; the prompt, only on the
//! output the [`super::LockLink`] names, is `ds::LockPrompt` with [`super::lock_user`] over
//! `use_auth().account`, [`super::prompt_state`] over `use_auth().phase`, [`super::caps_lock`]
//! over the last key event's modifiers, and `onsubmit` sending
//! `AuthCommand::Submit(Secret::new(text))` on `Services.auth_control` (the text is dropped
//! there, never stored). On `Accepted` it fades and, on the prompt's output, calls
//! `shell_host::use_lock().unlock()` when the fade settles; `shell_host::use_lock_phase()`
//! saying `Unlocking` first (the daemon's backstop) skips the fade. Style nothing under
//! `.ds-lock*` (quire CONSUMING "Lock, polkit and switcher").

use dioxus::prelude::*;
use shell_host::OutputInfo;

pub(super) fn lock_screen(output: OutputInfo) -> Element {
    let _ = output;
    rsx! {}
}
