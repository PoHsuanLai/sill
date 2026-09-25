//! The power menu's render: a `Ds` root (the active Space's look, `power_menu.material`) holding
//! a quire `Scrim` over the output and a `Sheet` with the four action buttons in
//! `SessionAction::POWER_MENU` order, each unavailable where `Session::offer` says so; follows
//! the shared [`super::PowerMenuLink`] onto the surface's visibility, remounts the sheet (its
//! `peek-in` entrance, focus on [`super::initial`]) per opening, and runs [`super::step`] on
//! the keys.

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The panel on `output`.
///
/// M5 freeze stub, filled by M5 agent (d) power-menu (FINDINGS "M5 freeze"). It renders
/// nothing rather than `todo!()`: the daemon spawns this surface warm on every output today,
/// hidden, and a panic in its first render would take the surface down with it.
pub(crate) fn power_menu(output: OutputInfo) -> Element {
    let _ = output;
    rsx! {}
}
