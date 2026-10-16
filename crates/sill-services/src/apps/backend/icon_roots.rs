//! Where an absolute `Icon=` may point (effects: the filesystem).
//!
//! A desktop entry is another program's text. The shell draws a file only from the folders the
//! host serves (`Dirs::icon_file_roots`, the same list the host's file roots begin with), so the
//! scan keeps an absolute `Icon=` only when the file really lies in one of them: a path outside,
//! or one that gets in through a link or `..`, is no icon. The check is the host's own (whole path
//! components, links resolved), made here early so a refused icon is not kept to draw nothing.

use std::path::{Path, PathBuf};

/// The folders an absolute `Icon=` may name a file in, resolved to their real places.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IconRoots(Vec<PathBuf>);

impl IconRoots {
    /// These folders. A relative path names no fixed place, and a folder that is not there holds
    /// nothing; both are dropped.
    pub fn new(roots: impl IntoIterator<Item = PathBuf>) -> IconRoots {
        IconRoots(
            roots
                .into_iter()
                .filter(|root| root.is_absolute())
                .filter_map(|root| root.canonicalize().ok())
                .collect(),
        )
    }

    /// Whether the file at `path` is in one of the folders: its real place (links and `..`
    /// resolved) starts with a root's, whole components compared, so `/opt-evil/i.png` is not in
    /// `/opt`. A path that is not there is in none.
    pub fn holds(&self, path: &Path) -> bool {
        path.canonicalize()
            .is_ok_and(|real| self.0.iter().any(|root| real.starts_with(root)))
    }
}
