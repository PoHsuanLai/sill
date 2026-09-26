//! sill's own lock screen (design/20 §1.9; M11 lane a; FINDINGS "M11 freeze" F544): one
//! `ext-session-lock-v1` surface per output, which shell-host creates when the daemon locks
//! (`crates/sill/src/lock/`), never a layer surface. Each draws quire's `LockScreen` stage in
//! `Ds { material: Window, extent: Viewport }`: the output's pre-blurred wallpaper crop
//! ([`backdrop::backdrop_for`]), quire's `LockClock` ([`view::clock_lines`]) and, on the output
//! [`LockLink`] names, quire's `LockPrompt` ([`view::lock_user`], [`view::prompt_state`],
//! [`view::caps_lock`]). quire's field holds the text and hands it over only in `onsubmit`, which
//! the surface wraps in a `Secret` at once and sends as `AuthCommand::Submit`; it is never kept
//! in a signal. Escape (quire's) empties the field.
//!
//! When `Auth.phase` becomes `Accepted` every lock surface fades out (design/20 §1.9: `fade`
//! `--t-move --e-exit`, quire's motion timer) and the prompt's calls shell-host's
//! `use_lock().unlock()` when its fade settles; the daemon unlocks as a backstop if that never
//! comes. A surface never decides a password itself.
//!
//! Keyboard: a lock surface always has it (the compositor gives it to one of them); the daemon
//! moves [`LockLink`] to whichever lock surface gains focus, so the prompt is where typing goes.

pub mod backdrop;
pub mod link;
pub mod view;

mod screen;

pub use backdrop::backdrop_for;
pub use link::{LockLink, PromptOutput};
pub use view::{ClockLines, caps_lock, clock_lines, lock_user, prompt_state};

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The lock screen on `output`. Needs a [`LockLink`] root context beside the daemon's usual
/// ones (the daemon's lock factory gives it one) and shell-host's lock handle (shell-host gives
/// every lock document one).
#[component]
pub fn LockScreen(output: OutputInfo) -> Element {
    screen::lock_screen(output)
}
