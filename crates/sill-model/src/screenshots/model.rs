//! The screenshots the thumbnail follows.

use std::path::PathBuf;

use super::tool::ShotKind;

/// A screenshot's id, counting up from 1 for the daemon's life.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ShotId(pub u64);

/// The small image the thumbnail draws.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ShotPreview {
    /// Being made: the thumbnail waits for it (a few tens of ms for a 4K PNG).
    Pending,
    /// A downscaled PNG in `$XDG_CACHE_HOME/sill/shots/` ([`super::preview_name`]).
    Ready(PathBuf),
    /// The preview could not be made: draw the file itself.
    Original,
}

/// The screenshot the thumbnail shows.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shot {
    pub id: ShotId,
    /// The file, as saved.
    pub path: PathBuf,
    pub preview: ShotPreview,
}

/// What the folders' watch is doing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Watching {
    /// Nothing: passive, or `screenshot.folders = "off"`.
    #[default]
    Off,
    /// These folders (the ones that exist).
    Folders(Vec<PathBuf>),
    /// The watch could not start; the reason, for `sill debug` and the log.
    Failed(String),
}

/// Whether the tool is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Taking {
    #[default]
    Idle,
    Running(ShotKind),
}

/// What the thumbnail follows.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Shots {
    /// The newest screenshot not yet dismissed; a newer one replaces it (its file stays).
    pub current: Option<Shot>,
    pub watching: Watching,
    pub taking: Taking,
    /// The last id given.
    pub last_id: ShotId,
}
