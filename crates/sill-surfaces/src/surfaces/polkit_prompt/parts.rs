//! The adapter onto quire's M11 `PolkitPrompt` (announced on quire's `m11-parts`, not landed at
//! the freeze: the avatar, the name, the message, the secret field, Cancel and Authenticate on a
//! `Sheet`, the details, the shake). Only this file changes when it lands; until then it draws
//! nothing (coherence rule 3).

#![allow(dead_code)] // Freeze: lane b mounts it from `panel.rs`.

use dioxus::prelude::*;

use super::view::PolkitPromptView;

/// quire's `PolkitPrompt`: `on_answer` gets the typed secret, `on_cancel` the cancel.
#[component]
pub(super) fn Prompt(
    view: PolkitPromptView,
    on_answer: EventHandler<String>,
    on_cancel: EventHandler<()>,
) -> Element {
    let _ = (view, on_answer, on_cancel);
    rsx! {}
}
