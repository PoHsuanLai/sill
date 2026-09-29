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
//! `current`. `screenshot.thumbnail = "hide"` keeps it hidden. The pure parts: [`step::step`],
//! [`drag::drag_end`], [`picture::png_size`] and [`layer::layer`].

mod card;
mod drag;
mod follow;
mod ghost;
mod io;
mod layer;
mod picture;
mod step;
mod surface;
mod view;

pub use surface::ScreenshotThumb;
pub use view::SCREENSHOT_THUMB_CSS;
