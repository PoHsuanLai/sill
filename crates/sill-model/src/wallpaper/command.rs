//! What the wallpaper service can be told.

use serde::{Deserialize, Serialize};

use crate::ids::OutputName;

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
    /// Re-read the source.
    Reload,
}
