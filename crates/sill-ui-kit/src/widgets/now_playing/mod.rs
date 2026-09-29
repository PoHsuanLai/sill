//! The Now Playing widget (kind `sill.now-playing`): the control center's Now Playing reused, not
//! redrawn. [`face`] is the current track as data and its one view, shared with the control
//! center's module; [`io`] reads the current player and sends a press to it.

mod face;
mod io;
#[cfg(test)]
mod tests;
mod widget;

/// This widget's stylesheet, in [`crate::widgets::widgets_css`]. The control center's own
/// stylesheet carries the module's layout rules, which the widgets' hosts do not load, so this
/// one carries the same few rules for the widget's classes.
pub const CSS: &str = include_str!("style.css");

pub use face::{MediaFace, MediaView, TrackFace, TrackTimes, Transport, media_face, position_of};
pub use io::{NowPlayingProvider, use_media};
pub use widget::NowPlaying;
