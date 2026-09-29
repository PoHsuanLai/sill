//! What the screenshots service hears and what it can be told.

use std::path::PathBuf;

use super::{ShotId, ShotKind, ShotTool, Watching};

/// Which folders are watched.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ShotFolders {
    /// None: `screenshot.folders = "off"`, and the passive start.
    #[default]
    Off,
    /// The Pictures folder (`$XDG_PICTURES_DIR`, else `~/Pictures`) and its `Screenshots`.
    Auto,
    /// These folders.
    At(Vec<PathBuf>),
}

/// What the daemon tells the service from the settings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ShotPolicy {
    pub folders: ShotFolders,
    pub tool: ShotTool,
}

/// What the screenshots service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShotCommand {
    /// Watch and take per `policy` from now on; the first one ends the passive start.
    Configure(ShotPolicy),
    /// `sill screenshot KIND`: run the tool (ignored while one is still running).
    Take(ShotKind),
    /// `sill debug screenshot PATH`: as if the watch had just seen `PATH` (any name, any folder).
    Announce(PathBuf),
    /// The thumbnail went away, and why: opened, kept, trashed or dragged out.
    Leave(ShotId, ShotLeave),
}

/// Why a thumbnail went away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShotLeave {
    /// Clicked: the file opens in its default app (`xdg-open`).
    Opened,
    /// Swiped away, or its hold ran out: the file stays where it was saved.
    Kept,
    /// Its Delete action: the file goes to the trash.
    Trashed,
    /// Dragged out and dropped as a copy (shell-host's drag source): the file stays.
    Dragged,
    /// Dragged out and dropped as a move: the target has taken the file, so nothing here
    /// touches it again.
    Moved,
}

/// What happens to the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShotEvent {
    /// A new screenshot file (the watch, or `Announce`).
    Appeared(PathBuf),
    PreviewReady(ShotId, PathBuf),
    PreviewFailed(ShotId),
    /// The thumbnail of `id` went away.
    Left(ShotId, ShotLeave),
    /// `Take` asked for a tool run.
    Take(ShotKind),
    /// The tool exited (whatever it saved arrives as `Appeared`).
    TakeEnded,
    Watching(Watching),
}

/// What the reducer asks the task to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShotEffect {
    /// Downscale `from` into the cache and answer `PreviewReady` or `PreviewFailed`.
    MakePreview { id: ShotId, from: PathBuf },
    /// `xdg-open` the file.
    Open(PathBuf),
    /// Move the file to the trash (`gio trash`).
    Trash(PathBuf),
    /// Run the tool for this kind, and answer `TakeEnded` when it exits.
    RunTool(ShotKind),
}
