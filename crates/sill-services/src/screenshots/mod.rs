//! Screenshots: noticing a new one, so the floating thumbnail can show it (design/20 §1.13), and
//! taking one on `sill screenshot` (sill FINDINGS "M10 freeze").
//!
//! How sill learns a screenshot happened (researched for the freeze): COSMIC's `cosmic-screenshot`
//! (1.8.0) asks the portal (`org.freedesktop.portal.Screenshot`, served by
//! xdg-desktop-portal-cosmic); the portal's picker saves the image as
//! `Screenshot_%Y-%m-%d_%H-%M-%S.png` in the Pictures folder (its `save_location`: Pictures,
//! Documents or the clipboard), or with `--interactive=false` cosmic-screenshot saves it into
//! `--save-dir` itself. Neither announces the file on the bus: the portal's `Response` goes to
//! the caller only, and `--notify` posts an ordinary notification. So the one route that sees
//! every screenshot, whoever took it (COSMIC's Print key, `sill screenshot`, spectacle, grim), is
//! **an inotify watch of the screenshot folders** (`notify`, already a dependency): a new file
//! whose name reads as a screenshot ([`names::is_screenshot_name`]), seen when it is closed after
//! writing or renamed into place, becomes the current [`Shot`]. `sill screenshot
//! [area|window|screen]` only runs the tool ([`tool::tool_command`]); its file arrives through
//! the same watch, so there is one path to test. A clipboard-only screenshot has no file and no
//! thumbnail, as on macOS.
//!
//! The service is passive until the daemon's first [`ShotCommand::Configure`] (from
//! `screenshot.folders` and `screenshot.tool`): a test or a headless render never watches
//! `~/Pictures` or runs a tool. Tests configure temp folders and a stand-in tool.

pub mod names;
pub mod reduce;
pub mod tool;

mod child;
mod folder_watch;
mod live;
mod preview;

pub use names::{ShotName, folders_of, is_screenshot_name, parse_folders, preview_name};
pub use reduce::{
    Shot, ShotEffect, ShotEvent, ShotId, ShotLeave, ShotPreview, Shots, Taking, Watching,
    reduce_shots,
};
pub use tool::{CommandLine, ShotKind, ShotTool, parse_tool, save_folder, tool_command};

use std::path::PathBuf;

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

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

/// Start the screenshots task on `rt`, passive until `Configure`.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Shots>, Control<ShotCommand>) {
    let (tx, rx) = watch::channel(Shots::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(tx, commands));
    (rx, control)
}
