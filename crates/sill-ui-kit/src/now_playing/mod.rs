//! Now Playing: the current track as data and its one view ([`face`]), drawn by the control
//! center's module, and the hook that reads the current player and sends a press to it ([`io`]).

mod face;
mod io;
#[cfg(test)]
mod tests;

pub use face::{MediaFace, MediaView, TrackFace, TrackTimes, Transport, media_face, position_of};
pub use io::use_media;
