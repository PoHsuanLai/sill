//! Resolving an entry's `Icon=` against the icon theme (effects: the filesystem).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sill_model::apps::{AppIcon, icon_file_fits};

/// The icon theme lookups use.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IconTheme(pub String);

/// The largest size a theme lookup asks for. 08-ICONS.md §2.2's master export is 1024, but no
/// installed icon theme ships raster tiles anywhere near that; 256 is the common ceiling
/// (Adwaita, Papirus, hicolor all top out there) and `freedesktop_icons` falls back to the
/// closest size it has when a theme has nothing this large.
const MAX_SIZE: u16 = 256;

/// The theme GTK's settings name (`gtk-icon-theme-name` in `gtk-3.0/settings.ini` under the
/// config directory), else `hicolor`. Every lookup falls back to the `hicolor` theme anyway
/// whenever the chosen theme is not installed (`freedesktop_icons::lookup`'s own step).
pub fn icon_theme(config: &Path) -> IconTheme {
    gtk_icon_theme(config).unwrap_or_else(|| IconTheme(HICOLOR.to_owned()))
}

/// The theme every lookup falls back to.
const HICOLOR: &str = "hicolor";

/// The path [`icon_theme`] reads under the config directory `config`, which the apps service
/// also watches for a live theme change.
pub fn icon_theme_path(config: &Path) -> PathBuf {
    config.join("gtk-3.0").join("settings.ini")
}

/// The `gtk-icon-theme-name` line's value. `None` when the file is absent or unreadable or
/// names no theme, which is exactly when the caller should fall back.
fn gtk_icon_theme(config: &Path) -> Option<IconTheme> {
    let text = std::fs::read_to_string(icon_theme_path(config)).ok()?;
    theme_named(&text)
}

fn theme_named(settings: &str) -> Option<IconTheme> {
    settings.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        let name = value.trim();
        (key.trim() == "gtk-icon-theme-name" && !name.is_empty())
            .then(|| IconTheme(name.to_owned()))
    })
}

/// `icon` (a name or an absolute path) as a file, the largest size up to [`MAX_SIZE`] px,
/// preferring an SVG over a PNG/XPM of the same or a smaller size. An absolute path is the entry's
/// own word for where its icon is, and is used when the file there is a regular file with an image
/// extension within the size cap (`sill_model::apps::icon_file_fits`): the entry is another
/// program's text, so a pipe, a folder, a document or a huge file is no icon. Whether the shell
/// may draw from the folder it is in is the host's list of roots (`Dirs::icon_dirs` and
/// `Dirs::app_stores`); a name goes through `freedesktop_icons` at `theme`, caching every lookup
/// (`freedesktop_icons`'s own process-wide cache).
pub fn resolve_icon(icon: &str, theme: &IconTheme) -> AppIcon {
    if icon.is_empty() {
        return AppIcon::None;
    }
    let as_path = Path::new(icon);
    if as_path.is_absolute() {
        return named_icon_file(as_path);
    }
    svg_lookup(icon, theme)
        .or_else(|| raster_lookup(icon, theme))
        .map_or(AppIcon::None, AppIcon::File)
}

/// `path` as the icon it names, when it is one (see [`resolve_icon`]).
fn named_icon_file(path: &Path) -> AppIcon {
    match std::fs::metadata(path) {
        Ok(meta) if meta.is_file() && icon_file_fits(path, meta.len()) => {
            AppIcon::File(path.to_owned())
        }
        _ => AppIcon::None,
    }
}

/// An SVG at (or covering) [`MAX_SIZE`], if the theme has one: `force_svg` still falls back to
/// a raster inside whichever directory the size search lands on, so the result is checked for
/// its own extension rather than trusted.
fn svg_lookup(icon: &str, theme: &IconTheme) -> Option<PathBuf> {
    let found = freedesktop_icons::lookup(icon)
        .with_theme(&theme.0)
        .with_size(MAX_SIZE)
        .with_cache()
        .force_svg()
        .find()?;
    (found.extension().is_some_and(|ext| ext == "svg")).then_some(found)
}

/// The largest PNG/XPM available, when no SVG was found.
fn raster_lookup(icon: &str, theme: &IconTheme) -> Option<PathBuf> {
    freedesktop_icons::lookup(icon)
        .with_theme(&theme.0)
        .with_size(MAX_SIZE)
        .with_cache()
        .find()
}

/// The directory a theme lookup falls back to for a pixmap, after every theme
/// (`freedesktop_icons`'s own hard-coded last resort, mirrored here so a caller can decide
/// whether an icon came from a theme or this fallback without re-deriving it).
pub fn pixmap_dirs() -> Vec<PathBuf> {
    vec![PathBuf::from("/usr/share/pixmaps")]
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{IconTheme, icon_theme_path, pixmap_dirs, resolve_icon, theme_named};
    use sill_model::apps::{AppIcon, ICON_FILE_MAX_BYTES};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    // SVG-vs-raster preference against a real fixture theme directory is `tests/icons.rs` (an
    // integration test): `freedesktop_icons`' theme table is a process-wide lazy static seeded
    // from `$HOME`/XDG env once, and this workspace denies `unsafe_code` (CONVENTIONS §2), so
    // there is no sound way to repoint it from inside this binary's own unit tests. See that
    // file's doc comment.

    /// A fresh directory under the system temp dir, removed on drop (no `tempfile`: not in the
    /// pinned block; same pattern as `sill-settings/tests/common`).
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            static NEXT: AtomicU32 = AtomicU32::new(0);
            let n = NEXT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("sill-services-{tag}-{}-{n}", std::process::id()));
            let _ = fs::remove_dir_all(&path);
            fs::create_dir_all(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            TempDir(path)
        }

        fn path(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn an_absolute_icon_is_used_only_when_it_is_an_image_file_of_a_sane_size() {
        let temp = TempDir::new("icons-abs");
        let theme = IconTheme("Fixture".to_owned());
        let write = |name: &str, len: u64| {
            let file = temp.path().join(name);
            fs::File::create(&file)
                .and_then(|file| file.set_len(len))
                .unwrap_or_else(|e| panic!("{e}"));
            file
        };
        let png = write("app.png", 3);
        let svg = write("app.SVG", 3);
        let note = write("app.txt", 3);
        let bare = write("app", 3);
        let huge = write("huge.png", ICON_FILE_MAX_BYTES + 1);
        let folder = temp.path().join("folder.png");
        fs::create_dir(&folder).unwrap_or_else(|e| panic!("{e}"));
        let link = temp.path().join("link.png");
        std::os::unix::fs::symlink(&png, &link).unwrap_or_else(|e| panic!("{e}"));
        let cases = [
            ("a png", png.clone(), AppIcon::File(png)),
            ("an svg, whatever the case", svg.clone(), AppIcon::File(svg)),
            ("a link to a png", link.clone(), AppIcon::File(link)),
            ("a document", note, AppIcon::None),
            ("no extension", bare, AppIcon::None),
            ("past the size cap", huge, AppIcon::None),
            ("a folder named like an image", folder, AppIcon::None),
            (
                "a file that is not there",
                temp.path().join("missing.png"),
                AppIcon::None,
            ),
        ];
        for (name, path, want) in cases {
            let icon = path.to_str().unwrap_or_else(|| panic!("utf8 path"));
            assert_eq!(resolve_icon(icon, &theme), want, "{name}");
        }
        assert_eq!(resolve_icon("", &theme), AppIcon::None);
    }

    #[test]
    fn the_icon_theme_is_the_one_gtk_settings_name() {
        assert_eq!(
            icon_theme_path(Path::new("/c")),
            Path::new("/c/gtk-3.0/settings.ini")
        );
        let cases = [
            (
                "named",
                "[Settings]\ngtk-icon-theme-name=Papirus\n",
                Some("Papirus"),
            ),
            (
                "spaced",
                "gtk-icon-theme-name = Adwaita \n",
                Some("Adwaita"),
            ),
            ("empty", "gtk-icon-theme-name=\n", None),
            ("absent", "[Settings]\ngtk-theme-name=Foo\n", None),
        ];
        for (name, text, want) in cases {
            assert_eq!(
                theme_named(text),
                want.map(|theme| IconTheme(theme.to_owned())),
                "{name}"
            );
        }
    }

    #[test]
    fn pixmap_dirs_is_the_one_freedesktop_icons_itself_falls_back_to() {
        assert_eq!(pixmap_dirs(), vec![PathBuf::from("/usr/share/pixmaps")]);
    }
}
