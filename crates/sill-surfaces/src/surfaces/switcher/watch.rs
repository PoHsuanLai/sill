//! The chord's modifiers as shell-host publishes them (`use_modifiers`, G180), fed to the
//! machine through [`SwitcherCtx::modifiers`]. Mounted per opening on a host surface only
//! (`use_modifiers` needs one) and only for the compositor's apps (`sill debug switcher`'s
//! made-up apps close on a key or a click, never on the state they opened in).
//!
//! The first read is the default (nothing held), before the compositor's `wl_keyboard.enter`
//! lands: it is skipped; every publish after it is the seat's real state.

use dioxus::prelude::*;
use shell_host::use_modifiers;

use super::drive::SwitcherCtx;
use super::hold::chord_of;

/// Whether the modifiers so far are the real state or the default before focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    Default,
    Published,
}

#[component]
pub(super) fn ChordWatch(ctx: SwitcherCtx) -> Element {
    let modifiers = use_modifiers();
    let mut read = use_hook(|| CopyValue::new(Read::Default));
    use_effect(move || {
        let chord = chord_of(&modifiers());
        if *read.peek() == Read::Default {
            read.set(Read::Published);
            return;
        }
        ctx.modifiers(chord);
    });
    rsx! {}
}
