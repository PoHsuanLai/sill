//! Each launcher surface follows the shared [`Reveal<LauncherOpening>`] onto its own visibility
//! (`SurfaceHandle::set_visibility`): the surfaces stay alive, warm, while hidden (plan "Launcher
//! v1"), and neither needs the other's id.

use dioxus::prelude::*;
use shell_host::{OutputId, SurfaceHandle, Visibility, use_watch};

use sill_ui_kit::links::{LauncherLink, LauncherOpening, Reveal};

/// The shared state, as this surface's signal.
pub(crate) fn use_shown(link: &LauncherLink) -> ReadSignal<Reveal<LauncherOpening>> {
    use_watch(link.subscribe())
}

/// Map this surface while the launcher shows on `output`, unmap it otherwise. Nothing to do
/// without a host (a test, an SSR render).
pub(crate) fn use_follow_visibility(shown: ReadSignal<Reveal<LauncherOpening>>, output: OutputId) {
    let surface = try_use_context::<SurfaceHandle>();
    let mut applied = use_hook(|| CopyValue::new(None::<Visibility>));
    use_effect(move || {
        let visible = shown.read().shows_on(output);
        if *applied.peek() == Some(visible) {
            return;
        }
        applied.set(Some(visible));
        if let Some(surface) = &surface {
            surface.set_visibility(visible);
        }
    });
}
