//! The power menu (design/20 §1.8): one Overlay surface per output, kept warm and hidden like
//! the launcher, that shows a full-output quire `Scrim` under a centred `Sheet` holding Log Out,
//! Restart, Shut Down and Suspend as quire `Button`s (`Primary` for the focused one, `Danger`
//! for Shut Down and Restart, `Secondary` otherwise) with `Kbd` hints.
//! Keyboard `Exclusive`: arrows move, Enter confirms, Escape closes; a scrim click closes.
//!
//! Opened by `sill power-menu {open,toggle}`, the bar's app menu ("Shut Down…"), and the
//! launcher's Log Out, Restart and Shut Down rows (focused on that row's action). All of them
//! write one shared [`crate::links::PowerMenuLink`] (`links`), which every output's surface follows onto
//! its own visibility; the daemon also feeds it the surface's keyboard focus (a focus loss after
//! holding focus closes it, as the launcher's does). A chosen action goes to
//! `sill_model::session::SessionCommand::Run`.

mod backend;
mod follow;
mod io;
mod keymap;
mod keys;
mod labels;
mod layer;
mod sheet;
mod surface;
mod view;

pub use surface::PowerMenu;
pub use view::POWER_MENU_CSS;
