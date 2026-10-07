//! Shell surfaces' parts: the lock and polkit prompts, the app switcher, the bar and dock pieces,
//! the OSD, control-center modules, notifications, thumbnails, now playing, the idle dim, the
//! user's picture, animated emoji, the month grid, clocks, batteries, and the desktop widgets with
//! their catalog, over `ds` (the generic components and the document seam), and the shell's own
//! tokens and sheets, which `kits()` and `stylesheet()` add to the design system's and to
//! `ds_shell`'s (the app-facing parts).

pub mod bar;
pub mod battery;
pub mod catalog;
pub mod clock;
pub mod control_center;
pub mod date_picker;
pub mod dock;
pub mod emoji;
pub mod idle_dim;
pub(crate) mod kept;
pub(crate) mod kit;
pub mod lock;
pub mod month_grid;
pub mod notifications;
pub mod now_playing;
pub mod osd;
pub(crate) mod sheets;
#[cfg(test)]
mod stored_words;
pub mod switcher;
pub mod thumbs;
pub mod tokens;
pub mod user_picture;
pub mod widget;

pub mod prelude;

// The curated roots: the shell's stylesheet assembly. Every other name is in `prelude` or at its
// home path.
pub use crate::kit::{KIT, component_sheets, kits, stylesheet};
