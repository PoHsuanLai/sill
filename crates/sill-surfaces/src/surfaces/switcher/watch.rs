//! The chord's modifiers as shell-host publishes them (`use_modifiers`, G180), fed to the
//! machine through [`SwitcherCtx::modifiers`]. Mounted per opening on a host surface only
//! (`use_modifiers` needs one) and only for the compositor's apps (`sill debug switcher`'s
//! made-up apps close on a key or a click, never on the state they opened in); mounting marks
//! the opening followed (`hold.rs`: the show delay then ends a tap that never read held).
//!
//! The first read, when it says nothing is held, is the value before this opening's focus (a
//! surface's modifiers reset when it loses the keyboard): it is skipped; a held chord, and every
//! publish after it, is the seat's real state.

use dioxus::prelude::*;
use shell_host::use_modifiers;

use super::drive::SwitcherCtx;
use super::hold::chord_of;
use super::keys::Chord;

/// Whether the modifiers so far are the real state or the value before the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    Before,
    Published,
}

#[component]
pub(super) fn ChordWatch(ctx: SwitcherCtx) -> Element {
    let modifiers = use_modifiers();
    use_hook(move || ctx.watched());
    let mut read = use_hook(|| CopyValue::new(Read::Before));
    use_effect(move || {
        let chord = chord_of(&modifiers());
        let before = *read.peek();
        read.set(Read::Published);
        match (before, chord) {
            (Read::Before, Chord::Released) => {}
            (Read::Before | Read::Published, _) => ctx.modifiers(chord),
        }
    });
    rsx! {}
}
