//! The desktop's context menu (a press of the secondary button on the wallpaper): Change
//! Wallpaper..., a `Menu{Context}` popup of the wallpaper's surface
//! ([`sill_ui_kit::menu::use_context_menu`]) opened at the pointer. Which entries it has is
//! [`model`], and [`io`] opens it (at [`sill_ui_kit::menu::PopupGeom::AtPoint`]) and opens Settings at its
//! Wallpaper page when the entry is picked.

mod io;
mod model;

pub(super) use io::{secondary_press, use_desktop_menu};

/// A point on the wallpaper in whole logical px.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct DesktopPoint {
    pub(super) x: i32,
    pub(super) y: i32,
}
