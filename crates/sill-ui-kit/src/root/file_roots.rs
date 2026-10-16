//! The folders the host lets a document read pictures from (`HostConfig::file_roots`).
//!
//! The host serves a `file:` URL only inside these, so a picture a document names from anywhere
//! else (a path a bus caller sent, a link in a page) never loads. Every place the shell draws a
//! file lives here, and the list comes from [`Dirs`], never from the process.

use std::path::PathBuf;

use sill_services::env::Dirs;

/// Where AccountsService keeps the pictures people chose for their accounts.
const ACCOUNT_PICTURES: &str = "/var/lib/AccountsService/icons";

/// The folders and files the shell draws from, in the order of the table in ARCHITECTURE.md:
/// the icon themes and pixmaps ([`Dirs::icon_dirs`]); sill's own cache (the tinted app icons,
/// the wallpaper and its blurred copy, the now-playing covers, the screenshot previews); the
/// Pictures folder (a screenshot before its preview is made); the person's `~/.face` and the
/// account pictures folder.
fn file_root_paths(dirs: &Dirs) -> Vec<PathBuf> {
    let mut roots = dirs.icon_dirs();
    roots.push(dirs.sill_cache());
    roots.extend(dirs.pictures.clone());
    roots.push(dirs.home.join(".face"));
    roots.push(PathBuf::from(ACCOUNT_PICTURES));
    roots
}

/// [`file_root_paths`] as the host takes them.
pub fn file_roots(dirs: &Dirs) -> shell_host::FileRoots {
    shell_host::FileRoots::new(file_root_paths(dirs))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use sill_services::env::Dirs;

    use super::file_root_paths;

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn the_roots_are_the_icon_folders_sills_cache_pictures_the_face_and_account_pictures() {
        let scratch = Dirs::under(Path::new("/s"));
        let with_pictures = Dirs {
            pictures: Some(PathBuf::from("/s/home/Pictures")),
            ..scratch.clone()
        };
        let cases: [(&str, Dirs, Vec<PathBuf>); 2] = [
            (
                "no Pictures folder",
                scratch,
                paths(&[
                    "/s/data/icons",
                    "/s/data/pixmaps",
                    "/s/share/icons",
                    "/s/share/pixmaps",
                    "/s/home/.icons",
                    "/usr/share/pixmaps",
                    "/s/cache/sill",
                    "/s/home/.face",
                    "/var/lib/AccountsService/icons",
                ]),
            ),
            (
                "a Pictures folder",
                with_pictures,
                paths(&[
                    "/s/data/icons",
                    "/s/data/pixmaps",
                    "/s/share/icons",
                    "/s/share/pixmaps",
                    "/s/home/.icons",
                    "/usr/share/pixmaps",
                    "/s/cache/sill",
                    "/s/home/Pictures",
                    "/s/home/.face",
                    "/var/lib/AccountsService/icons",
                ]),
            ),
        ];
        for (name, dirs, want) in cases {
            assert_eq!(file_root_paths(&dirs), want, "{name}");
        }
    }
}
