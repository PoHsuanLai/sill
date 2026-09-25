//! Where quire's app icon files are (effects: the environment and the filesystem, read once).
//!
//! quire exposes no path for its `assets/icons/apps` (sill FINDINGS "Icon style", quire gap), so
//! the first of these that holds the set wins:
//!
//! 1. `$SILL_ICON_ASSETS`, for a packager or a test;
//! 2. `quire/icons/apps` under the XDG data directories (`$XDG_DATA_HOME`, then
//!    `$XDG_DATA_DIRS` or `/usr/local/share:/usr/share`), where an install would put it;
//! 3. the sibling quire checkout this crate was built against (`../quire`, the same layout the
//!    workspace's path dependencies assume), for a development build.
//!
//! None of them: no shipped icons, every app keeps its theme icon.

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use crate::app_icons::plan::AssetDir;

/// The variable that names the icon directory outright.
pub const ASSETS_ENV: &str = "SILL_ICON_ASSETS";

/// The file whose presence says a directory holds the set.
const PROBE: &str = "terminal/48.png";

static FOUND: LazyLock<Option<AssetDir>> = LazyLock::new(find);

/// quire's app icon directory, found once per process.
pub fn asset_dir() -> Option<&'static AssetDir> {
    FOUND.as_ref()
}

fn find() -> Option<AssetDir> {
    let env = std::env::var_os(ASSETS_ENV).map(PathBuf::from);
    let data = dirs::data_dir().into_iter().chain(data_dirs());
    let dev = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../quire/assets/icons/apps");
    candidates(env, data, dev)
        .into_iter()
        .find(|dir| dir.join(PROBE).is_file())
        .map(|dir| AssetDir(dir.canonicalize().unwrap_or(dir)))
}

/// `$XDG_DATA_DIRS`, or its specified default.
fn data_dirs() -> Vec<PathBuf> {
    let raw = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|dirs| !dirs.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());
    raw.split(':')
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .collect()
}

/// The directories to try, in order. Pure.
fn candidates(
    env: Option<PathBuf>,
    data: impl IntoIterator<Item = PathBuf>,
    dev: PathBuf,
) -> Vec<PathBuf> {
    env.into_iter()
        .chain(
            data.into_iter()
                .map(|dir| dir.join("quire").join("icons").join("apps")),
        )
        .chain([dev])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{PROBE, candidates, find};
    use std::path::PathBuf;

    #[test]
    fn the_variable_then_the_data_dirs_then_the_checkout() {
        let got = candidates(
            Some(PathBuf::from("/opt/icons")),
            [
                PathBuf::from("/home/u/.local/share"),
                PathBuf::from("/usr/share"),
            ],
            PathBuf::from("/src/quire/assets/icons/apps"),
        );
        let want: Vec<PathBuf> = [
            "/opt/icons",
            "/home/u/.local/share/quire/icons/apps",
            "/usr/share/quire/icons/apps",
            "/src/quire/assets/icons/apps",
        ]
        .into_iter()
        .map(PathBuf::from)
        .collect();
        assert_eq!(got, want);
        assert_eq!(
            candidates(None, [], PathBuf::from("/d")),
            vec![PathBuf::from("/d")]
        );
    }

    /// The development build finds the sibling checkout's set (the workspace cannot build
    /// without `../quire`, so it is there).
    #[test]
    fn a_development_build_finds_the_sibling_checkout() {
        let dir = find().unwrap_or_else(|| panic!("no quire app icons found"));
        assert!(dir.0.join(PROBE).is_file(), "{}", dir.0.display());
    }
}
