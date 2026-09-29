//! The wallpapers' state: where each comes from and every output's prepared images.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ids::OutputName;

/// Where a wallpaper comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum WallpaperSource {
    /// One image for both schemes.
    File(PathBuf),
    /// A light and a dark image, cross-faded when the scheme changes.
    Pair { light: PathBuf, dark: PathBuf },
    /// No image configured: the surface paints the `--paper` token.
    None,
}

/// Every output's prepared images.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Wallpapers {
    pub outputs: Vec<WallpaperImage>,
}

/// One output's cached images, at its buffer size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WallpaperImage {
    pub output: OutputName,
    pub light: PathBuf,
    pub dark: PathBuf,
}
