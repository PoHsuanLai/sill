//! Now Playing: the MPRIS players, their tracks, art and transport, as the control center draws
//! them.

mod art;
mod art_cache;
mod art_pace;
mod command;
mod limits;
mod model;
mod step;
#[cfg(test)]
mod tests;

pub use art::{ART_URL_MAX_CHARS, ArtSource, art_cache_name, art_source, local_art_cache_name};
pub use art_cache::{CacheLimits, CachedCover, evictions};
pub use art_pace::next_start;
pub use command::{MediaCommand, MediaEvent, MediaVerb};
pub use limits::{ARTISTS_MAX, PLAYERS_MAX, TEXT_MAX_CHARS};
pub use model::{Abilities, Art, Media, MediaPlayer, Micros, Playback, PlayerId, Timeline, Track};
pub use step::step;
