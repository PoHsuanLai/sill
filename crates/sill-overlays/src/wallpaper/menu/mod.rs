//! The desktop's context menu (a press of the secondary button on the wallpaper): Change
//! Wallpaper..., a `Menu{Context}` popup of the wallpaper's surface
//! ([`sill_ui_kit::menu::context_menu_popup`]) opened at the pointer. Which entries it has is
//! [`model`], where the popup goes is [`geometry`], and [`io`] opens it and launches the
//! wallpaper picker, the first app of [`model::PICKERS`] the app index has. Without one the menu
//! has no entries and does not open.

mod geometry;
mod io;
mod model;

pub(super) use io::{secondary_press, use_desktop_menu};
pub(super) use model::MenuWorld;

/// A point on the wallpaper in whole logical px.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct DesktopPoint {
    pub(super) x: i32,
    pub(super) y: i32,
}
