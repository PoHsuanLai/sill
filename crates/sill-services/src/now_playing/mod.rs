//! Now Playing: every MPRIS player on the session bus (`org.mpris.MediaPlayer2.*`) and which one
//! the control center shows (design/20 §1.5, design/13 §13.3.7: the Now Playing module).
//!
//! Spoken through hand-written `zbus` proxies for `org.mpris.MediaPlayer2` and
//! `org.mpris.MediaPlayer2.Player`, found by `ListNames` and followed by `NameOwnerChanged`
//! (the `mpris` crate runs on the `dbus` crate and libdbus). Album art reaches the surface only as
//! a local file: a `file://` URL is used as it is, an `http(s)://`
//! one is downloaded once into `$XDG_CACHE_HOME/sill/art/` ([`art`]), since quire's `IconUrl`
//! takes `file:` and `data:` only.

pub mod art;
mod live;
mod model;
mod mpris;

pub use art::{ArtSource, art_cache_name, art_source};
pub use model::{
    Abilities, Art, Media, MediaEvent, MediaPlayer, Micros, Playback, PlayerId, Timeline, Track,
    reduce_media,
};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// One transport button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaVerb {
    PlayPause,
    Next,
    Previous,
}

/// What the Now Playing service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MediaCommand {
    /// Press `verb` on `player` (the one the surface showed, so a player that took over in
    /// between is not the one that pauses). A verb the player cannot do is dropped.
    Press { player: PlayerId, verb: MediaVerb },
    /// Re-read every player (after a resume).
    Refresh,
    /// Re-read `player`'s `Position` (and the track's `mpris:length`) and republish its
    /// timeline: a read, nothing sent to the player. MPRIS signals no position change as a
    /// track plays, so a surface that opens on a player asks for where it is now.
    Position(PlayerId),
}

/// Start the Now Playing task on `rt`. Without a session bus the receiver stays at
/// `Media::default()` (no players), and the module shows nothing playing.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Media>, Control<MediaCommand>) {
    let (tx, rx) = watch::channel(Media::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(tx, commands));
    (rx, control)
}
