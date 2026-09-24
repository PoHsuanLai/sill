//! Resolving an entry's `Icon=` against the icon theme (effects: the filesystem).

#![allow(unused_variables, dead_code)] // Freeze stubs

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::apps::index::AppIcon;

/// The icon theme lookups use.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IconTheme(pub String);

/// COSMIC's `icon_theme` (`com.system76.CosmicTk/v1/icon_theme`), else `Cosmic`, else
/// `hicolor` (plan hooks table).
pub fn icon_theme() -> IconTheme {
    todo!("M2 apps-index: read ~/.config/cosmic/com.system76.CosmicTk/v1/icon_theme")
}

/// `icon` (a name or an absolute path) as a file, the largest size up to 256 px.
pub fn resolve_icon(icon: &str, theme: &IconTheme) -> AppIcon {
    todo!("M2 apps-index: freedesktop_icons::lookup(icon).with_theme(..).with_size(..)")
}

/// The directories a theme lookup falls back to for a pixmap (`/usr/share/pixmaps`).
pub fn pixmap_dirs() -> Vec<PathBuf> {
    todo!("M2 apps-index")
}
