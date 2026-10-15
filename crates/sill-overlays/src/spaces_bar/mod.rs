//! Mission Control's Spaces bar (what the Mac shows across the top of the screen while Mission
//! Control is open): one Top surface per output, kept warm and hidden, that appears when the
//! compositor opens Mission Control ([`sill_ui_kit::links::SpacesBarLink`]) across the strip at
//! the top of the output the compositor keeps free of windows. It slides down and fades in as
//! the windows spread, and back up as they return. It lists the output's Spaces by name
//! ("Desktop 1", and the apps of a full-screen or Split View Space), the one on show marked, with
//! a plus at the right edge. A click on a Space writes the pick to the link; the daemon switches
//! to it and closes Mission Control. The plus adds a Space.
//!
//! No keyboard: the compositor owns Mission Control's keys.

mod io;
mod layer;
mod surface;
mod view;

pub use surface::SpacesBar;
pub use view::SPACES_BAR_CSS;
