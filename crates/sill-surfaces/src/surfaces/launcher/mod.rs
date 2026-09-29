//! The launcher (design/20 §1.3, design/13 §13.3.9, 06 §20.2): two surfaces kept warm with
//! `set_visibility` — a transparent full-output catcher (painted once, never animated) and the
//! centred 680 x 460 panel with keyboard exclusivity — closing on Escape, focus lost, a catcher
//! click or `launcher toggle`.
//!
//! Both surfaces follow one shared [`LauncherLink`] (`presence.rs`), which the daemon writes
//! for `sill launcher toggle|open|close` and the panel's focus, and each surface maps or
//! unmaps itself from. The panel also writes there which part of it takes presses
//! ([`hit::PanelHit`]: its content-sized card), which the catcher's input leaves out.

pub mod activate;
pub mod chord_keys;
pub mod debug_link;
pub mod entries;
pub mod groups;
pub mod hit;
pub mod keymap;
pub mod keys;
pub mod layer;
pub mod list_detail;
pub mod pane_session;
pub mod presence;
pub mod preview;
pub mod row_icon;
pub mod rows;
pub mod script;
pub mod sections_view;
pub mod session;
pub mod view;

mod card;
mod catcher;
mod claim_keys;
mod contact;
mod events;
mod follow;
mod pane_keys;
pub(crate) mod panel;
mod run;

pub use activate::{LauncherEffect, LauncherProviders, effect_of};
pub use keys::{LauncherKey, LauncherOut, LauncherState, step};
pub use layer::{
    CARD_BLUR_RADIUS, CARD_ID, CARD_INSET, CARD_RADIUS, PANEL_HEIGHT, PANEL_ROOM_WIDTH,
    PANEL_WIDTH, SHADOW_BELOW, SHADOW_SIDE, catcher_input, catcher_layer, panel_layer, panel_rect,
    panel_top,
};
pub use presence::{LauncherLink, LauncherShown, Opening};

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The panel on `output`: quire `CommandPalette` with the search field, results grouped by
/// provider, and the actions menu (Ctrl+K / Alt+K). Needs a [`LauncherLink`] root context
/// beside the daemon's usual ones.
#[component]
pub fn Launcher(output: OutputInfo) -> Element {
    panel::launcher(output)
}

/// The catcher on `output`: an empty, transparent, full-output surface whose click closes the
/// launcher. Needs a [`LauncherLink`] root context.
#[component]
pub fn LauncherCatcher(output: OutputInfo) -> Element {
    catcher::catcher(output)
}
