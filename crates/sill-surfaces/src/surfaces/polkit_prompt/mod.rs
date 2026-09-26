//! sill's own polkit prompt (design/20 §1.10; M11 lane b; FINDINGS "M11 freeze" F526): one
//! Overlay per output, kept warm and hidden like the power menu, shown on the main display (the
//! banners' `BannerLink` output) while `use_polkit().current` holds a request: a full-output
//! quire `Scrim` under quire's `PolkitPrompt` on a centred `Sheet` (the avatar and name of the
//! identity asked for, the action's message, the helper's prompt as the field's placeholder,
//! Cancel and Authenticate, the details in a `HoverCard`), `peek-in` in and `shake-x` once on a
//! wrong password ([`view::prompt_view`]). Keyboard `Exclusive`: Enter authenticates, Escape
//! cancels ([`view::polkit_key`]); a scrim click cancels too.
//!
//! An answer goes to the agent as `PolkitCommand::Answer`, a cancel as `Dismiss`; the surface
//! never talks to polkit. quire's `PolkitPrompt` has not landed; `parts.rs` is the one file
//! that names it (F530).

pub mod layer;
pub mod view;

mod panel;
mod parts;

pub use layer::{PANEL_ID, polkit_layer};
pub use view::{
    Busy, PolkitKey, PolkitKeyOut, PolkitPromptView, placeholder_of, polkit_key, prompt_view,
};

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The polkit prompt on `output`. Needs the daemon's `BannerLink` root context (only the output
/// it names shows the prompt).
#[component]
pub fn PolkitPrompt(output: OutputInfo) -> Element {
    panel::polkit_prompt(output)
}
