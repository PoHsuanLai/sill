//! Where a notification's picture may come from. Any bus caller names `app_icon` and
//! `image-path`; a path is drawn only when it lies in one of these folders (the icon themes and
//! pixmaps, sill's own notification cache), so a sender cannot make the shell open an arbitrary
//! file. A symlink inside a folder that leads out of it is refused where the file is read (the
//! host's file roots); this check is about the path as written.

use std::path::{Component, Path, PathBuf};

use super::content::{AppRef, IconRef, NoteImage, Posting};

/// The folders a notification's picture may be read from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImageDirs(Vec<PathBuf>);

impl ImageDirs {
    /// These folders. A relative one names no fixed place and is left out.
    pub fn new(dirs: impl IntoIterator<Item = PathBuf>) -> ImageDirs {
        ImageDirs(dirs.into_iter().filter(|dir| dir.is_absolute()).collect())
    }

    /// Whether `path` is absolute, free of `.` and `..`, and inside one of the folders.
    pub fn admits(&self, path: &Path) -> bool {
        path.is_absolute()
            && path
                .components()
                .all(|part| !matches!(part, Component::CurDir | Component::ParentDir))
            && self.0.iter().any(|dir| path.starts_with(dir))
    }
}

impl IconRef {
    /// This icon if it may be drawn: a name is a name, with no folder in it; a file must be
    /// inside `dirs`. Anything else is no icon.
    pub fn within(self, dirs: &ImageDirs) -> IconRef {
        match self {
            IconRef::File(path) if dirs.admits(&path) => IconRef::File(path),
            IconRef::Name(name) if !name.contains('/') => IconRef::Name(name),
            IconRef::File(_) | IconRef::Name(_) | IconRef::None => IconRef::None,
        }
    }
}

impl Posting {
    /// This posting with every picture a bus caller named kept only where [`ImageDirs`] allow it:
    /// `app_icon` and `image-path` that fail are no picture; `image-data` is pixels and stays.
    pub fn within(self, dirs: &ImageDirs) -> Posting {
        let Posting {
            replaces,
            app,
            summary,
            body,
            actions,
            mut hints,
            expiry,
        } = self;
        hints.image = match hints.image {
            NoteImage::Icon(icon) => match icon.within(dirs) {
                IconRef::None => NoteImage::None,
                icon => NoteImage::Icon(icon),
            },
            other => other,
        };
        Posting {
            replaces,
            app: AppRef {
                icon: app.icon.within(dirs),
                ..app
            },
            summary,
            body,
            actions,
            hints,
            expiry,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::ImageDirs;
    use crate::notifications::{AppRef, Body, Expiry, Hints, IconRef, NoteImage, Pixels, Posting};

    fn dirs() -> ImageDirs {
        ImageDirs::new([
            PathBuf::from("/usr/share/icons"),
            PathBuf::from("/home/p/.cache/sill/notifications"),
            PathBuf::from("relative/is/left/out"),
        ])
    }

    #[test]
    fn an_icon_is_kept_only_when_it_is_a_plain_name_or_a_file_in_a_folder() {
        let file = |path: &str| IconRef::File(PathBuf::from(path));
        let name = |text: &str| IconRef::Name(text.to_owned());
        let cases: Vec<(&str, IconRef, IconRef)> = vec![
            ("none", IconRef::None, IconRef::None),
            ("a theme name", name("mail-unread"), name("mail-unread")),
            ("a name with a folder", name("a/b"), IconRef::None),
            ("a name climbing out", name("../../etc/x"), IconRef::None),
            (
                "a theme file",
                file("/usr/share/icons/hicolor/48x48/apps/a.png"),
                file("/usr/share/icons/hicolor/48x48/apps/a.png"),
            ),
            (
                "the notification cache",
                file("/home/p/.cache/sill/notifications/1.png"),
                file("/home/p/.cache/sill/notifications/1.png"),
            ),
            (
                "a secret file",
                file("/home/p/.ssh/id_ed25519"),
                IconRef::None,
            ),
            ("a temporary file", file("/tmp/a.png"), IconRef::None),
            (
                "a sibling folder with the same prefix",
                file("/usr/share/icons-evil/a.png"),
                IconRef::None,
            ),
            (
                "a path climbing out of a folder",
                file("/usr/share/icons/../../../etc/passwd"),
                IconRef::None,
            ),
            ("a relative path", file("icons/a.png"), IconRef::None),
            (
                "the folder itself",
                file("/usr/share/icons"),
                file("/usr/share/icons"),
            ),
        ];
        for (case, icon, want) in cases {
            assert_eq!(icon.within(&dirs()), want, "{case}");
        }
    }

    #[test]
    fn a_posting_loses_a_picture_outside_the_folders_and_keeps_pixels() {
        let posting = |icon: IconRef, image: NoteImage| Posting {
            replaces: None,
            app: AppRef {
                name: "app".to_owned(),
                icon,
                desktop_entry: None,
            },
            summary: String::new(),
            body: Body::default(),
            actions: Vec::new(),
            hints: Hints {
                image,
                ..Hints::default()
            },
            expiry: Expiry::Default,
        };
        let outside = IconRef::File(PathBuf::from("/etc/shadow"));
        let inside = IconRef::File(PathBuf::from("/usr/share/icons/a.png"));
        let pixels = NoteImage::Pixels(Pixels {
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 255],
        });
        let cases: Vec<(&str, Posting, (IconRef, NoteImage))> = vec![
            (
                "both outside",
                posting(outside.clone(), NoteImage::Icon(outside.clone())),
                (IconRef::None, NoteImage::None),
            ),
            (
                "both inside",
                posting(inside.clone(), NoteImage::Icon(inside.clone())),
                (inside.clone(), NoteImage::Icon(inside.clone())),
            ),
            (
                "pixels with an outside icon",
                posting(outside, pixels.clone()),
                (IconRef::None, pixels),
            ),
        ];
        for (case, before, (icon, image)) in cases {
            let after = before.within(&dirs());
            assert_eq!((after.app.icon, after.hints.image), (icon, image), "{case}");
        }
    }
}
