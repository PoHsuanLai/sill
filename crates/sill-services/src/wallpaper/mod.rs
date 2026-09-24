//! The wallpaper: one pre-resized PNG per output size and scheme, cached in
//! `$XDG_CACHE_HOME/sill/wallpaper/<hash>-<w>x<h>.png` (plan "Surfaces": "pre-resized PNG cache
//! ... via image 0.25 Lanczos3; light/dark cross-fade over t-big; renders only on change").
//!
//! design/22 has no key naming the wallpaper file; M2 reads COSMIC's own background config
//! (`com.system76.CosmicBackground`) until one is added (FINDINGS.md).

#![allow(unused_variables, dead_code)] // Freeze stubs

mod cache;

pub use cache::{CACHE_DIR, cache_file_name, source_hash};

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;
use crate::cosmic_wl::OutputName;

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

/// Start the wallpaper service: resize on the blocking pool, publish cached paths.
pub fn spawn(rt: &Handle) -> (watch::Receiver<Wallpapers>, Control<WallpaperCommand>) {
    todo!("M2 wallpaper: image::imageops::resize(.., Lanczos3) into CACHE_DIR, publish")
}
