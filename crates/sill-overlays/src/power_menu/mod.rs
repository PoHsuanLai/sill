//! The power menu (design/20 §1.8): one Overlay surface per output, kept warm and hidden like
//! the launcher, that shows quire's `Alert` centred on its `Sheet` (it dims nothing): the
//! question for the opening's action, then Log Out, Restart, Shut Down and Suspend as its buttons
//! (the opening's first and the default, the ones logind does not offer left out) and Cancel.
//! Keyboard `Exclusive`: Return presses the default button, Tab and Shift+Tab move between the
//! buttons, Space presses the one under the keyboard, Escape cancels; a press outside the alert closes.
//!
//! Opened by `sill power-menu {open,toggle}`, the bar's app menu ("Shut Down…"), and the
//! launcher's Log Out, Restart and Shut Down rows (the alert is about that row's action). All of them
//! write one shared [`sill_ui_kit::links::PowerMenuLink`] (`links`), which every output's surface follows onto
//! its own visibility; the daemon also feeds it the surface's keyboard focus (a focus loss after
//! holding focus closes it, as the launcher's does). A chosen action goes to
//! `sill_model::session::SessionCommand::Run`.

mod backend;
mod choice;
mod follow;
mod io;
mod labels;
mod layer;
mod sheet;
mod surface;
mod view;

pub use surface::PowerMenu;
pub use view::POWER_MENU_CSS;
