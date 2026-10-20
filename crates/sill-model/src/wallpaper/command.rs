//! What the wallpaper service can be told.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::OutputName;

/// The `wallpaper.picture` setting as the service is told it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum WallpaperPicture {
    /// The key is empty: the person has not chosen a picture through Settings.
    Unset,
    /// The picture the person chose.
    Chosen(PathBuf),
}

/// What the wallpaper service can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum WallpaperCommand {
    /// An output appeared or changed size: prepare its images (buffer pixels).
    Prepare {
        output: OutputName,
        width: u32,
        height: u32,
    },
    /// An output went away.
    Forget(OutputName),
    /// The `wallpaper.picture` setting (sent at start and whenever it changes).
    Picture(WallpaperPicture),
    /// Re-read the source.
    Reload,
}
