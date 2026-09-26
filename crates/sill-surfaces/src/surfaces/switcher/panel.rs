//! The switcher surface's root (lane c fills it; it draws nothing until then).
//!
//! What it does once filled: follows the [`super::SwitcherLink`], acting on each new ask for its
//! own output; on a chord it maps the surface (`use_surface().set_visibility`) so it takes the
//! keyboard, computes the apps with `sill_services::switcher_apps` over `use_toplevels()` and
//! `use_apps()` (or the ask's made-up apps), and steps [`super::machine::step`] with
//! `SwitcherParams::new(&settings.switcher)`; the show delay's `Tick` comes from quire's timer
//! (`ds::sleep`), never a tokio sleep; `Show` fades the panel in, `Select` moves its ring,
//! `Activate`/`Quit`/`HideApp` send `sill_services::switcher::{activate, quit, hide}` over
//! `Services.wl` (a `Quit` marks the tile `TilePresence::Leaving` so it folds, and drops it at
//! `settle(Anim::Fold)`, the panel staying up), `Hide` unmaps at once; key-downs through
//! [`super::sw_key`], key-ups through [`super::chord_after_keyup`] (a `Released` is
//! `ModifierReleased`); the panel is `ds::AppSwitcher` from [`super::tiles`], `selected` the
//! machine's selection as [`super::key_of`], `metrics` [`super::metrics_of`], `output`
//! [`super::output_width`], and `onhover` / `onactivate` as `Hover` / `Click` through
//! [`super::index_of`]. Style nothing under `.ds-switcher*`.

use dioxus::prelude::*;
use shell_host::OutputInfo;

pub(super) fn switcher_panel(output: OutputInfo) -> Element {
    let _ = output;
    rsx! {}
}
