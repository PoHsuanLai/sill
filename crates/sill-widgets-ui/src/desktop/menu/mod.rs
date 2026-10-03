//! The desktop's context menu (a press of the secondary button on the wallpaper or between the
//! widgets): Change Wallpaper..., Edit Widgets... and Show View Options, a `Menu{Context}` popup
//! of the layer ([`sill_ui_kit::menu::context_menu_popup`]) opened at the pointer. Which
//! entries it has is [`model`], what a pick asks for is [`pick`], where the popup goes is
//! [`geometry`], and [`io`] opens it and carries the asks out: Edit Widgets opens on this layer's
//! own output, the wallpaper picker is the first app of [`model::PICKERS`] the app index has,
//! and View Options write the grid's cell and gap through the Edit Widgets link's writer.

mod geometry;
mod io;
mod model;
mod pick;

pub(super) use io::use_desktop_menu;
pub(super) use model::MenuWorld;

#[cfg(test)]
mod popup_tests;
