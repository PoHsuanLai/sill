//! The adapter onto quire's M11 lock parts (announced on quire's `m11-parts`, not landed at the
//! freeze): `LockClock` and `LockPrompt` (avatar, name, secret field, states
//! Idle/Checking/Wrong/LockedOut, the caps-lock mark, the shake). Only this file changes when
//! they land: [`Clock`] and [`Prompt`] take sill's own view types and hand them to quire's
//! components; until then they draw nothing (coherence rule 3: nothing missing is patched
//! locally).

#![allow(dead_code)] // Freeze: lane a mounts these from `screen.rs`.

use dioxus::prelude::*;
use sill_services::auth::Account;

use super::view::{CapsLock, ClockLines, PromptState};

/// quire's `LockClock`.
#[component]
pub(super) fn Clock(lines: ClockLines) -> Element {
    let _ = lines;
    rsx! {}
}

/// quire's `LockPrompt`: `on_submit` gets the typed secret.
#[component]
pub(super) fn Prompt(
    account: Account,
    state: PromptState,
    caps: CapsLock,
    on_submit: EventHandler<String>,
) -> Element {
    let _ = (account, state, caps, on_submit);
    rsx! {}
}
