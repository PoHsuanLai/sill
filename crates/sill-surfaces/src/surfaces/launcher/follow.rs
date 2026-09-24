//! Each launcher surface follows the shared [`LauncherShown`] onto its own visibility
//! (`SurfaceOp::SetVisibility`): the surfaces stay alive, warm, while hidden (plan "Launcher
//! v1"), and neither needs the other's id.

use dioxus::prelude::*;
use shell_host::{OutputId, SurfaceHandle, SurfaceOp, Visibility, use_watch};

use crate::surfaces::launcher::presence::{LauncherLink, LauncherShown, Visible};

/// The shared state, as this surface's signal.
pub(crate) fn use_shown(link: &LauncherLink) -> ReadSignal<LauncherShown> {
    use_watch(link.subscribe())
}

/// Map this surface while the launcher shows on `output`, unmap it otherwise. Nothing to do
/// without a host (a test, an SSR render).
pub(crate) fn use_follow_visibility(shown: ReadSignal<LauncherShown>, output: OutputId) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(None::<Visible>));
    use_effect(move || {
        let visible = shown.read().shows_on(output);
        if *applied.peek() == Some(visible) {
            return;
        }
        applied.set(Some(visible));
        if let Some(surface) = &surface {
            surface.apply(SurfaceOp::SetVisibility(match visible {
                Visible::Yes => Visibility::Shown,
                Visible::No => Visibility::Hidden,
            }));
        }
    });
}
