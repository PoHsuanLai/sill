//! The launcher (design/20 §1.3, design/13 §13.3.9, 06 §20.2): two surfaces kept warm with
//! `set_visibility` — a transparent full-output catcher (`catcher/`, painted once, never
//! animated) and the centred 680 x 460 panel with keyboard exclusivity — closing on Escape,
//! focus lost, a catcher click or `launcher toggle`.
//!
//! Both surfaces follow one shared [`sill_ui_kit::links::LauncherLink`] (`links`), which the daemon writes
//! for `sill launcher toggle|open|close` and the panel's focus, and each surface maps or
//! unmaps itself from. The panel also writes there which part of it takes presses
//! ([`sill_ui_kit::links::PanelHit`]: its content-sized card), which the catcher's input leaves out.

pub mod action_keys;
pub mod activate;
pub mod companion;
pub mod entries;
pub mod groups;
pub mod hit;
pub mod keymap;
pub mod keys;
pub mod pane_session;
pub mod preview;
pub mod row_icon;
pub mod row_thumb;
pub mod rows;
pub mod sections_view;
pub mod session;
pub mod view;

mod card;
mod catcher;
mod claim_keys;
mod contact;
mod events;
mod follow;
mod helpers;
mod layer;
mod mode_switch;
mod pane_keys;
pub(crate) mod panel;
mod picking;
mod run;
mod surface;
pub mod tokens;

pub use activate::{LauncherEffect, LauncherProviders, effect_of};
pub use catcher::LauncherCatcher;
pub use keys::{LauncherKey, LauncherOut, LauncherState, step};
pub use layer::{
    CARD_BLUR_RADIUS, CARD_ID, CARD_INSET, CARD_RADIUS, PANEL_HEIGHT, PANEL_ROOM_WIDTH,
    PANEL_WIDTH, SHADOW_BELOW, SHADOW_SIDE, panel_rect, panel_top,
};
pub use panel::LAUNCHER_CSS;
pub use surface::Launcher;
