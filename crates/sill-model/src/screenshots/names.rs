//! Which files are screenshots, which folders are watched, and where a preview is cached: pure.

use std::path::{Path, PathBuf};

use super::ShotFolders;

/// Whether a file name reads as a finished screenshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShotName {
    Screenshot,
    Other,
}

/// A finished screenshot's name: it starts with `Screenshot` (any case: the portal's
/// `Screenshot_2026-09-26_10-15-02.png`, GNOME's `Screenshot From 2026-09-26 10-15-02.png`,
/// spectacle's `Screenshot_20260926_101502.png`) and ends in `.png`, `.jpg`, `.jpeg` or `.webp`;
/// hidden, partial (`.part`, `.tmp`, `~`) and other names are `Other`.
pub fn is_screenshot_name(name: &str) -> ShotName {
    let lower = name.to_lowercase();
    let named = lower.starts_with("screenshot");
    let image = IMAGE_ENDINGS.iter().any(|ending| lower.ends_with(ending));
    match (named, image) {
        (true, true) => ShotName::Screenshot,
        _ => ShotName::Other,
    }
}

/// The endings a finished screenshot has (a partial file's `.part`, `.tmp` or `~` is none).
const IMAGE_ENDINGS: [&str; 4] = [".png", ".jpg", ".jpeg", ".webp"];

/// `screenshot.folders`: `auto` and `off` (any case, trimmed; empty is `off`), else
/// `;`-separated folders, `~/` expanded against `home`.
pub fn parse_folders(text: &str, home: &Path) -> ShotFolders {
    let trimmed = text.trim();
    match trimmed.to_lowercase().as_str() {
        "auto" => return ShotFolders::Auto,
        "off" | "" => return ShotFolders::Off,
        _ => {}
    }
    let folders: Vec<PathBuf> = trimmed
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| expand_home(part, home))
        .collect();
    match folders.is_empty() {
        true => ShotFolders::Off,
        false => ShotFolders::At(folders),
    }
}

/// `part` with a leading `~` (alone, or as `~/`) read as `home`.
fn expand_home(part: &str, home: &Path) -> PathBuf {
    match part.strip_prefix('~') {
        Some("") => home.to_path_buf(),
        Some(rest) if rest.starts_with('/') => home.join(rest.trim_start_matches('/')),
        _ => PathBuf::from(part),
    }
}

/// The folders to watch: `Auto` is `pictures` and `pictures/Screenshots` (none without a
/// Pictures folder), `At` its own, `Off` none. Existence is the caller's to check.
pub fn folders_of(folders: &ShotFolders, pictures: Option<&Path>) -> Vec<PathBuf> {
    match (folders, pictures) {
        (ShotFolders::Auto, Some(pictures)) => {
            vec![pictures.to_path_buf(), pictures.join("Screenshots")]
        }
        (ShotFolders::At(folders), _) => folders.clone(),
        (ShotFolders::Auto, None) | (ShotFolders::Off, _) => Vec::new(),
    }
}

/// The cache file name of `shot`'s preview: stable across builds for one path and its size and
/// mtime (not std's `DefaultHasher`), ending in `.png`.
pub fn preview_name(shot: &Path, size: u64, modified_ms: i64) -> String {
    let hash = fnv1a(FNV_OFFSET_BASIS, shot.as_os_str().as_encoded_bytes());
    let hash = fnv1a(hash, &size.to_le_bytes());
    let hash = fnv1a(hash, &modified_ms.to_le_bytes());
    format!("{hash:016x}.png")
}

/// FNV-1a (64-bit), fixed where std's hasher is seeded per process: a preview keeps its name
/// across restarts (the wallpaper cache's choice, for the same reason).
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

fn fnv1a(hash: u64, bytes: &[u8]) -> u64 {
    bytes.iter().fold(hash, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{ShotName, folders_of, is_screenshot_name, parse_folders, preview_name};
    use crate::screenshots::ShotFolders;

    #[test]
    fn screenshot_names_are_recognised() {
        let cases = [
            ("Screenshot_2026-09-26_10-15-02.png", ShotName::Screenshot),
            (
                "Screenshot From 2026-09-26 10-15-02.png",
                ShotName::Screenshot,
            ),
            ("Screenshot_20260926_101502.png", ShotName::Screenshot),
            ("screenshot.JPG", ShotName::Screenshot),
            ("Screenshot_x.png.part", ShotName::Other),
            (".Screenshot_x.png", ShotName::Other),
            ("Screenshot_x.png~", ShotName::Other),
            ("holiday.png", ShotName::Other),
            ("Screenshot_notes.txt", ShotName::Other),
        ];
        for (name, want) in cases {
            assert_eq!(is_screenshot_name(name), want, "{name}");
        }
    }

    #[test]
    fn the_folders_setting_parses_and_resolves() {
        let home = Path::new("/home/ada");
        assert_eq!(parse_folders(" Auto ", home), ShotFolders::Auto);
        assert_eq!(parse_folders("", home), ShotFolders::Off);
        assert_eq!(
            parse_folders("~/Shots; /srv/grabs", home),
            ShotFolders::At(vec![
                PathBuf::from("/home/ada/Shots"),
                PathBuf::from("/srv/grabs")
            ])
        );
        let pictures = Path::new("/home/ada/Pictures");
        assert_eq!(
            folders_of(&ShotFolders::Auto, Some(pictures)),
            vec![pictures.to_path_buf(), pictures.join("Screenshots")]
        );
        assert_eq!(folders_of(&ShotFolders::Auto, None), Vec::<PathBuf>::new());
        assert_eq!(
            folders_of(&ShotFolders::Off, Some(pictures)),
            Vec::<PathBuf>::new()
        );
    }

    #[test]
    fn a_preview_name_is_stable_and_changes_with_the_file() {
        let path = Path::new("/p/Screenshot_1.png");
        let one = preview_name(path, 10, 1_000);
        assert_eq!(one, preview_name(path, 10, 1_000));
        assert!(one.ends_with(".png"));
        assert_ne!(one, preview_name(path, 11, 1_000));
        assert_ne!(one, preview_name(path, 10, 2_000));
        assert_ne!(
            one,
            preview_name(Path::new("/p/Screenshot_2.png"), 10, 1_000)
        );
    }
}
