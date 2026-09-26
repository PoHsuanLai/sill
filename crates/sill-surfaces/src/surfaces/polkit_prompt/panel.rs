//! The polkit prompt's root (lane b fills it; it draws nothing until then).
//!
//! What it draws once filled: while `use_polkit().current` is a request and the `BannerLink`
//! names this output, the surface maps itself (`use_surface().set_visibility`), and a
//! `Ds { material: Sheet, extent: Viewport }` holds `Sheet { placement: Centre, scrim: Modal }`
//! with `parts::Prompt` from [`super::prompt_view`]; a new cookie remounts it (the entrance plays
//! again); keys through [`super::polkit_key`]; an answer sends `PolkitCommand::Answer` (the
//! field's text, then the field is cleared), a cancel `PolkitCommand::Dismiss`; when the request
//! goes away the sheet leaves and the surface unmaps at `on_hidden`.

use dioxus::prelude::*;
use shell_host::OutputInfo;

pub(super) fn polkit_prompt(output: OutputInfo) -> Element {
    let _ = output;
    rsx! {}
}
