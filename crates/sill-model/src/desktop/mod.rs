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
mod consent;
mod force_quit;
mod model;
mod note;
mod output_picture;
mod overview;
mod spaces_bar;
#[cfg(test)]
mod spaces_bar_tests;
mod split;
pub mod target;
#[cfg(test)]
mod target_tests;
#[cfg(test)]
mod tests;
mod trial;

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
    Answer, AppLifeKind, AppVisibility, AskId as ConsentId, Attention, Corner, DragOutcome,
    KeyGesture, Memory, OverviewMode, OverviewState, Power, ProofKind, ShellRole, Shown,
    SpaceKind, SplitSide, SubjectKind, TrialAnswer, TrialId, WindowEntries, WindowEntry, WireCode,
};
pub use command::{DesktopCommand, Supports, supports};
pub use consent::{Consent, ConsentSubject, SUBJECT_MAX_CHARS};
pub use force_quit::{ForceQuitRow, Standing, force_quit_rows, hung_first};
pub use model::{ActiveView, AppView, DesktopView, SpacePlaceView, SpaceView, WindowView};
pub use note::{OverviewNote, ShellNote, SwitcherNote};
pub use output_picture::{OutputPicture, OutputShot};
pub use overview::open_mission_control;
pub use spaces_bar::{
    BarPick, BarShow, BarSpace, bar_show, bar_space_ids, bar_spaces, commands_of,
};
pub use split::{SplitCard, cards_of, output_of};
pub use trial::{
    DisplayTrial, KEEP_LABEL, REVERT_LABEL, TRIAL_TITLE, countdown_line, seconds_left,
    until_next_second,
};
