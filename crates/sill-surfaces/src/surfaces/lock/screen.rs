//! The lock surface's root (lane a fills it; it draws nothing until then).
//!
//! What it draws once filled: `Ds { material: Window, extent: Viewport }` with the backdrop
//! image ([`super::backdrop_for`] over `use_wallpapers()`, the scheme from `use_appearance()`;
//! the `--paper` token when there is none), `parts::Clock` from [`super::clock_lines`] over the
//! bar clock's minute (`use_clock()` wakes it) and `session.lock_clock`; on the output the
//! [`super::LockLink`] names, `parts::Prompt` from `use_auth()` ([`super::prompt_state`]), the
//! caps-lock mark from the last key's modifiers ([`super::caps_lock`]), Enter and Escape
//! through [`super::lock_key`], a submit as `AuthCommand::Submit` on `Services.auth_control`.
//! The unlock's `fade` (design/20 §1.9) is shell-host's to play before it destroys the lock
//! surfaces, or none.

use dioxus::prelude::*;
use shell_host::OutputInfo;

pub(super) fn lock_screen(output: OutputInfo) -> Element {
    let _ = output;
    rsx! {}
}
