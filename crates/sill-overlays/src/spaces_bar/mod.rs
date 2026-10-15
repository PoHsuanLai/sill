//! Mission Control's Spaces bar (what the Mac shows across the top of the screen while Mission
//! Control is open): one Top surface per output, kept warm and hidden, that appears when the
//! compositor opens Mission Control on that output ([`sill_ui_kit::links::SpacesBarLink`]) and
//! hangs quire's `SpacesBar` under the menu bar. It drops in as the windows spread and lifts away
//! as they return. It lists the output's Spaces by name ("Desktop 1", and the apps of a
//! full-screen or Split View Space), each over a picture of the Space when the compositor has
//! made one (of every Space, hidden ones too, over the output's wallpaper), the one on show marked and the one a dragged window is over lit, with a plus at the
//! right edge. A click on a Space writes the pick to the link; the daemon switches to it and
//! closes Mission Control. The plus adds a Space, and lights while a dragged window is over it (a drop there makes a new Space). The surface also writes where its cards lie to
//! the link, which the daemon turns into the compositor's drop targets.
//!
//! No keyboard: the compositor owns Mission Control's keys.

mod io;
mod layer;
mod surface;
mod view;

pub use layer::STRIP_PX;
pub use surface::SpacesBar;
pub use view::SPACES_BAR_CSS;
