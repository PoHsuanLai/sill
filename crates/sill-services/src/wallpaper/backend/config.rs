//! The wallpaper's source: the `wallpaper.picture` setting, which the daemon sends as a
//! [`WallpaperPicture`].
//!
//! Migration: while the setting is empty ([`WallpaperPicture::Unset`]), the picture is the one the
//! old private file `sill/wallpaper-source` under the config directory names (its first line).
//! That file is only read here, never written or removed; the setting wins as soon as it is set.
//!
//! [`parse_source_path`] reads the old file's line; a blank file, or a path that is not a file on
//! disk, answers [`WallpaperSource::None`] through [`resolve_source`].

use std::path::{Path, PathBuf};

use sill_model::wallpaper::{WallpaperPicture, WallpaperSource};

/// The old file this reads while the setting is empty, under the config directory.
const RELATIVE_PATH: &str = "sill/wallpaper-source";

/// Where the old file is kept under the config directory `config`.
pub fn config_path(config: &Path) -> PathBuf {
    config.join(RELATIVE_PATH)
}

/// The path on the first line of `text`, trimmed. `None` for blank text.
pub fn parse_source_path(text: &str) -> Option<PathBuf> {
    let line = text.lines().next()?.trim();
    (!line.is_empty()).then(|| PathBuf::from(line))
}

/// The picture the old file names, if it exists and names one.
fn old_file_picture(config: &Path) -> Option<PathBuf> {
    std::fs::read_to_string(config_path(config))
        .ok()
        .and_then(|text| parse_source_path(&text))
}

/// The wallpaper source right now: the chosen picture, or (while none is chosen) the old file's,
/// when that is a file on disk; otherwise [`WallpaperSource::None`], which the surface paints as
/// `--paper`. A path [`image`] cannot decode (this crate's `image` build carries only
/// `png`/`jpeg`) is not distinguished here: it reaches the same fallback once the resize fails
/// (`resize::ensure_cached`'s `Err`).
pub fn resolve_source(picture: &WallpaperPicture, config: &Path) -> WallpaperSource {
    let path = match picture {
        WallpaperPicture::Chosen(path) => Some(path.clone()),
        WallpaperPicture::Unset => old_file_picture(config),
    };
    match path.filter(|path| path.is_file()) {
        Some(path) => WallpaperSource::File(path),
        None => WallpaperSource::None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::parse_source_path;

    #[test]
    fn every_case_extracts_or_refuses_the_path() {
        let cases: &[(&str, &str, Option<&str>)] = &[
            (
                "a bare path",
                "/usr/share/backgrounds/dark.png\n",
                Some("/usr/share/backgrounds/dark.png"),
            ),
            (
                "surrounding space",
                "  /a/b.png  \nignored\n",
                Some("/a/b.png"),
            ),
            ("blank first line", "\n/a/b.png\n", None),
            ("empty text", "", None),
        ];
        for (name, text, want) in cases {
            let got = parse_source_path(text);
            assert_eq!(got, want.map(PathBuf::from), "{name}");
        }
    }
}
