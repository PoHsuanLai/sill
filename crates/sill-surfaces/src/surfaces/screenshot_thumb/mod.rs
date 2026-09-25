//! The screenshot thumbnail (design/20 §1.13; M10 lane c): after a screenshot, its picture slides
//! in at the output's bottom right and stays `screenshot.thumbnail_hold_ms` (paused while the
//! pointer is on it); a click opens the file (`ShotLeave::Opened`, `xdg-open`), a swipe right
//! keeps it and dismisses the thumbnail (`Kept`), the hold running out does the same, its Delete
//! action trashes the file (`Trashed`), and a drag out of the thumbnail hands the file to
//! another app as `text/uri-list` ([`drag`], waiting on shell-host's drag source, G4).
//!
//! One warm Overlay per output, spawned hidden like the banners, shown only on the output the
//! banners' `BannerLink` names (the main display's stand-in), following `use_shots().current`.
//! `screenshot.thumbnail = "hide"` keeps it hidden. The pure parts: [`machine::step`] and
//! [`layer::thumb_layer`].

pub mod drag;
pub mod follow;
pub mod gesture;
pub mod layer;
pub mod machine;

mod card;
mod drive;
mod view;

pub use drag::{DragStart, start_file_drag};
pub use layer::{THUMB_ID, THUMB_RADIUS, THUMB_WIDTH, thumb_layer};
pub use machine::{ThumbIn, ThumbOut, ThumbParams, ThumbState, step};

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
