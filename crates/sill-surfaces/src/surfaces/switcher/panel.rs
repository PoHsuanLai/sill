//! The switcher surface's root (lane c fills it; it draws nothing until then).
//!
//! What it does once filled: follows the [`super::SwitcherLink`], acting on each new ask for its
//! own output; on a chord it maps the surface (`use_surface().set_visibility`) so it takes the
//! keyboard, computes the apps with `sill_services::switcher_apps` over `use_toplevels()` and
//! `use_apps()` (or the ask's made-up apps), and steps [`super::machine::step`] with
//! `SwitcherParams::new(&settings.switcher)`; the show delay's `Tick` comes from quire's timer
//! (`ds::sleep`), never a tokio sleep; `Show` fades `parts::Panel` in, `Select` moves its ring,
//! `Activate`/`Quit`/`HideApp` send `sill_services::switcher::{activate, quit, hide}` over
//! `Services.wl` (a `Quit` folds the tile away and keeps the panel up), `Hide` unmaps at once;
//! key-downs through [`super::sw_key`], key-ups through [`super::chord_after_keyup`] (a
//! `Released` is `ModifierReleased`), hover and click on a tile as `Hover`/`Click`.

use dioxus::prelude::*;
use shell_host::OutputInfo;

pub(super) fn switcher_panel(output: OutputInfo) -> Element {
    let _ = output;
    rsx! {}
}
