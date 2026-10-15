//! The desktop as the shell sees it, whichever compositor protocol carries it: which backend
//! the compositor's globals select, the views of its apps, windows and Spaces, the commands
//! the shell sends and whether a backend can carry each, and the notes the compositor's shell
//! protocol sends the shell. Pure; `sill-services` fills the views
//! from the wire.

mod backend;
mod bar_drops;
mod bar_layout;
mod bar_outbox;
mod bar_outputs;
#[cfg(test)]
mod bar_tests;
mod bar_thumbs;
mod command;
mod model;
mod note;
mod spaces_bar;
#[cfg(test)]
mod spaces_bar_tests;
mod split;
pub mod target;
#[cfg(test)]
mod target_tests;
#[cfg(test)]
mod tests;

pub use backend::{DesktopBackend, GlobalOffer};
pub use bar_drops::BarDrops;
pub use bar_layout::{
    BAR_EXPANDED_PX, BarLayout, BarLayouts, BarTile, THUMB_H, THUMB_W, target_commands, thumb_size,
};
pub use bar_outbox::{Outbox, outgoing};
pub use bar_outputs::BarOutputs;
pub use bar_thumbs::{
    BarPicture, BarPictures, SpacePicture, ThumbAsks, ThumbRequest, thumbs_wanted,
};
pub use casement_protocols::vocab::{
    Corner, KeyGesture, OverviewMode, OverviewState, ShellRole, Shown, SpaceKind, SplitSide,
    WindowEntry, WireCode,
};
pub use command::{DesktopCommand, Supports, supports};
pub use model::{ActiveView, AppView, DesktopView, SpacePlaceView, SpaceView, WindowView};
pub use note::{OverviewNote, ShellNote, SwitcherNote};
pub use spaces_bar::{
    BarPick, BarShow, BarSpace, bar_show, bar_space_ids, bar_spaces, commands_of,
};
pub use split::{SplitCard, cards_of, output_of};
