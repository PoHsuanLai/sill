//! The screenshot thumbnail (design/20 §1.13): after a screenshot, quire's `ShotThumbnail` rises in
//! at the output's bottom right and stays `screenshot.thumbnail_hold_ms` (paused while the pointer
//! is on it); a click opens the file (`ShotLeave::Opened`, `xdg-open`), a swipe right keeps it and
//! dismisses the thumbnail (`Kept`), the hold running out does the same, its Delete action trashes
//! the file (`Trashed`), and a drag out hands the file to another app as `text/uri-list` through
//! shell-host's drag source ([`drag`]): a copy dismisses it (`Dragged`), a move gives the file away
//! (`Moved`), a cancel leaves it.
//!
//! One warm Overlay per output, spawned hidden like the banners, shown only on the output the
//! banners' `BannerLink` names (the main display's stand-in), following the screenshots service's
//! `current`. `screenshot.thumbnail = "hide"` keeps it hidden. The pure parts: [`machine::step`],
//! [`drag::drag_end`], [`picture::png_size`] and [`layer::thumb_layer`].

pub mod drag;
pub mod follow;
pub mod ghost;
pub mod layer;
pub mod machine;
pub mod picture;

mod card;
mod drive;
mod view;

pub use layer::{THUMB_ID, THUMB_RADIUS, THUMB_WIDTH, thumb_layer};
pub use machine::{DragEnd, ThumbIn, ThumbOut, ThumbParams, ThumbState, step};

use dioxus::prelude::*;
use shell_host::OutputInfo;

/// The thumbnail's stylesheet.
pub const SCREENSHOT_THUMB_CSS: &str = include_str!("../../style/screenshot_thumb.css");

/// The thumbnail on `output`. Needs a [`BannerLink`](crate::surfaces::banners::BannerLink)
/// root context beside the daemon's usual ones (the daemon's `surface_root` gives every surface
/// one): only the output it names shows the thumbnail.
#[component]
pub fn ScreenshotThumb(output: OutputInfo) -> Element {
    view::thumb(output)
}
