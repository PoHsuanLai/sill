//! sill's own lock screen (design/20 §1.9; M11 lane a; FINDINGS "M11 freeze" F524): one
//! `ext-session-lock-v1` surface per output, which shell-host creates when the daemon locks
//! (`crates/sill/src/lock/`), never a layer surface. Each draws the output's pre-blurred
//! wallpaper crop ([`backdrop::backdrop_for`]) under a `Window`-material root and quire's
//! `LockClock`; the output [`LockLink`] names also draws quire's `LockPrompt`: the avatar, the
//! name, the password field, the caps-lock mark, the shake on a wrong password
//! ([`view::prompt_state`]). The field's Enter sends `AuthCommand::Submit` to the auth service;
//! the daemon unlocks when `Auth.phase` becomes `Accepted`, never the surface.
//!
//! Keyboard: a lock surface always has it (the compositor gives it to one of them); the daemon
//! moves [`LockLink`] to whichever lock surface gains focus, so the prompt is where typing goes.
//! Escape clears the field ([`keys::lock_key`]).
//!
//! quire's M11 parts (`LockClock`, `LockPrompt`) have not landed; `parts.rs` is the one file
//! that names them, and draws nothing until they do (FINDINGS F530).

pub mod backdrop;
pub mod keys;
pub mod link;
pub mod view;

mod parts;
mod screen;

pub use backdrop::backdrop_for;
pub use keys::{FieldText, LockKey, LockKeyOut, lock_key};
pub use link::{LockLink, PromptOutput};
pub use view::{CapsLock, ClockLines, PromptState, caps_lock, clock_lines, prompt_state};

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The lock screen on `output`. Needs a [`LockLink`] root context beside the daemon's usual
/// ones (the daemon's lock factory gives it one).
#[component]
pub fn LockScreen(output: OutputInfo) -> Element {
    screen::lock_screen(output)
}
