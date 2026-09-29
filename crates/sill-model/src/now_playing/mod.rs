//! Now Playing: the MPRIS players, their tracks, art and transport, as the control center draws them.

mod art;
mod command;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use art::{ArtSource, art_cache_name, art_source};
pub use command::{MediaCommand, MediaEvent, MediaVerb};
pub use model::{Abilities, Art, Media, MediaPlayer, Micros, Playback, PlayerId, Timeline, Track};
pub use step::step;
