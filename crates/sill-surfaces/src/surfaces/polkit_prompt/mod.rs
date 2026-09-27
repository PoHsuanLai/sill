//! sill's own polkit prompt (design/20 §1.10; M11 lane b; FINDINGS "M11 freeze" F546): one
//! Overlay per output, kept warm and hidden like the power menu, shown on the main display (the
//! banners' `BannerLink` output) while `use_polkit().current` holds a request: quire's
//! `PolkitPrompt` (a narrow `Sheet` over the modal scrim, `peek-in` in and `sheet-out` out: the
//! avatar and name of the identity asked for, the action's sentence, the details as a hover
//! card, a secret field, Cancel and Authenticate; `Wrong` shakes once) in `Ds { material: Sheet,
//! extent: Viewport }` ([`view::prompt_view`]). Keyboard `Exclusive`; Enter submits and Escape
//! and the scrim cancel, all inside quire's component.
//!
//! An answer goes to the agent as `PolkitCommand::Answer` (the text wrapped in a `Secret` at
//! once, never kept), a cancel as `Dismiss`; the surface never talks to polkit.

pub mod layer;
pub mod stage;
pub mod view;

mod panel;

pub use layer::{PANEL_ID, polkit_layer};
pub use stage::{Mapped, Stage, StageIn, advance};
pub use view::{PolkitPromptView, detail_of, prompt_view};

use dioxus::prelude::*;
use shell_host::OutputInfo;

use crate::host_modality::HostModalityBridge;

/// The polkit prompt on `output`. Needs the daemon's `BannerLink` root context (only the output
/// it names shows the prompt).
#[component]
pub fn PolkitPrompt(output: OutputInfo) -> Element {
    rsx! {
        HostModalityBridge { {panel::polkit_prompt(output)} }
    }
}
