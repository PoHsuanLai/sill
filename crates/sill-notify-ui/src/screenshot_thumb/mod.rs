//! The screenshot thumbnail (design/20 §1.13): after a screenshot, quire's `ShotThumbnail` rises in
//! at the output's bottom right and stays the thumbnail hold (paused while the pointer
//! is on it); a click opens the file (`ShotLeave::Opened`, `xdg-open`), a swipe right keeps it and
//! dismisses the thumbnail (`Kept`), the hold running out does the same, its Delete action trashes
//! the file (`Trashed`), and a drag out hands the file to another app as `text/uri-list` through
//! shell-host's drag source ([`drag`]): a copy dismisses it (`Dragged`), a move gives the file away
//! (`Moved`), a cancel leaves it.
//!
//! The card never takes the keyboard on appear. `sill focus-latest` (the launcher's kind of
//! shortcut) asks for it through the banners' link ([`sill_ui_kit::links::BannerLink`]);
//! the card then holds the keyboard and its hold pauses, Enter and Space open it, Delete and
//! Backspace trash it, Escape dismisses it (the file kept), Tab moves between its buttons; every
//! way out gives the keyboard back ([`step`], [`keys`]).
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
mod keys;
mod layer;
mod picture;
mod step;
mod surface;
mod view;

pub use surface::ScreenshotThumb;
pub use view::SCREENSHOT_THUMB_CSS;
