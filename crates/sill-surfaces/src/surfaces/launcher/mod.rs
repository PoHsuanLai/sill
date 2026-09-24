//! The launcher (design/20 §1.3, design/13 §13.3.9, 06 §20.2): two surfaces kept warm with
//! `set_visibility` — a transparent full-output catcher (painted once, never animated) and the
//! centred 680 x 460 panel with keyboard exclusivity — closing on Escape, focus lost, a catcher
//! click or `launcher toggle`.

#![allow(unused_variables, dead_code)] // Freeze stubs

pub mod actions;
pub mod keys;
pub mod layer;
pub mod results;

pub use keys::{LauncherKey, LauncherOut, LauncherState, step};
pub use layer::{PANEL_HEIGHT, PANEL_WIDTH, catcher_layer, panel_layer, panel_top};

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The panel on `output`: quire `CommandPalette` with the search field, results grouped by
/// provider, and the actions menu (Ctrl+K / Alt+K).
#[component]
pub fn Launcher(output: OutputInfo) -> Element {
    todo!("M4 launcher-ui")
}

/// The catcher on `output`: an empty, transparent, full-output surface whose click closes the
/// launcher.
#[component]
pub fn LauncherCatcher(output: OutputInfo) -> Element {
    todo!("M4 launcher-ui")
}
