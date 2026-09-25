//! Which files are screenshots, which folders are watched, and where a preview is cached: pure.

use std::path::{Path, PathBuf};

use super::ShotFolders;

/// Whether a file name reads as a finished screenshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShotName {
    Screenshot,
    Other,
}

/// A finished screenshot's name: it starts with `Screenshot` (any case: COSMIC's
/// `Screenshot_2026-09-26_10-15-02.png`, GNOME's `Screenshot From 2026-09-26 10-15-02.png`,
/// spectacle's `Screenshot_20260926_101502.png`) and ends in `.png`, `.jpg`, `.jpeg` or `.webp`;
/// hidden, partial (`.part`, `.tmp`, `~`) and other names are `Other`.
pub fn is_screenshot_name(name: &str) -> ShotName {
    let _ = name;
    todo!("M10 fill: c-screenshot")
}

/// `screenshot.folders`: `auto` and `off` (any case, trimmed; empty is `off`), else
/// `;`-separated folders, `~/` expanded against `home`.
pub fn parse_folders(text: &str, home: &Path) -> ShotFolders {
    let _ = (text, home);
    todo!("M10 fill: c-screenshot")
}

/// The folders to watch: `Auto` is `pictures` and `pictures/Screenshots` (none without a
/// Pictures folder), `At` its own, `Off` none. Existence is the caller's to check.
pub fn folders_of(folders: &ShotFolders, pictures: Option<&Path>) -> Vec<PathBuf> {
    let _ = (folders, pictures);
    todo!("M10 fill: c-screenshot")
}

/// The cache file name of `shot`'s preview: stable across builds for one path and its size and
/// mtime (not std's `DefaultHasher`), ending in `.png`.
pub fn preview_name(shot: &Path, size: u64, modified_ms: i64) -> String {
    let _ = (shot, size, modified_ms);
    todo!("M10 fill: c-screenshot")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{ShotName, folders_of, is_screenshot_name, parse_folders, preview_name};
    use crate::screenshots::ShotFolders;

    #[test]
    #[ignore = "M10 fill: c-screenshot"]
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
    #[ignore = "M10 fill: c-screenshot"]
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
    #[ignore = "M10 fill: c-screenshot"]
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
