//! The chord's modifiers as shell-host publishes them (`use_modifiers`), fed to the
//! machine through [`SwitcherCtx::modifiers`]. Mounted per opening on a host surface only
//! (`use_modifiers` needs one) and only for the compositor's apps (`sill debug switcher`'s
//! made-up apps close on a key or a click, never on the state they opened in); mounting marks
//! the opening followed (`hold.rs`: the show delay then ends a tap that never read held).
//!
//! shell-host publishes only a change. An unmap withdraws the keyboard and resets the modifiers,
//! so an opening normally mounts on "nothing held"; the value as it mounts is still taken as its
//! baseline, which keeps the switcher right on a host whose unmap does not reset them and costs
//! nothing now. A change after that is the seat's real state (a release, the
//! release).

use dioxus::prelude::*;
use shell_host::use_modifiers;

use super::hold::chord_of;
use super::io::SwitcherCtx;
use super::keys::Chord;

/// Whether the modifiers read so far are this opening's baseline or a publish after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Read {
    Baseline,
    Published,
}

#[component]
pub(super) fn ChordWatch(ctx: SwitcherCtx) -> Element {
    let modifiers = use_modifiers();
    let baseline = use_hook(move || {
        let baseline = chord_of(&modifiers.peek());
        ctx.watched(baseline);
        baseline
    });
    let mut read = use_hook(|| CopyValue::new(Read::Baseline));
    use_effect(move || {
        let chord = chord_of(&modifiers());
        let before = *read.peek();
        read.set(Read::Published);
        match (before, chord, baseline) {
            (Read::Baseline, Chord::Held, Chord::Held)
            | (Read::Baseline, Chord::Released, Chord::Released) => {}
            (Read::Baseline | Read::Published, _, _) => ctx.modifiers(chord),
        }
    });
    rsx! {}
}
