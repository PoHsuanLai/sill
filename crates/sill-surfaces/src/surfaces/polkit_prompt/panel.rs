//! The polkit prompt's root (lane b fills it; it draws nothing until then).
//!
//! What it draws once filled: while `use_polkit().current` is a request and the `BannerLink`
//! names this output, the surface maps itself (`use_surface().set_visibility`), and a
//! `Ds { material: Sheet, extent: Viewport }` holds `ds::PolkitPrompt` from
//! [`super::prompt_view`] (keyed by the cookie, so a new request enters again), `caps` from the
//! last key event's modifiers, `onsubmit` sending `PolkitCommand::Answer { cookie, secret:
//! Secret::new(text) }`, `oncancel` sending `PolkitCommand::Dismiss(cookie)`; when the request
//! goes away `shown` turns `Hidden` and the surface unmaps at `on_hidden`. Style nothing under
//! `.ds-polkit*` (quire CONSUMING "Lock, polkit and switcher").

use dioxus::prelude::*;
use shell_host::OutputInfo;

pub(super) fn polkit_prompt(output: OutputInfo) -> Element {
    let _ = output;
    rsx! {}
}
