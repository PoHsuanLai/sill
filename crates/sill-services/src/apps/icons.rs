//! Resolving an entry's `Icon=` against the icon theme (effects: the filesystem).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::apps::index::AppIcon;

/// The icon theme lookups use.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IconTheme(pub String);

/// The file [`icon_theme`] reads, and the apps service watches, for a live theme change.
const COSMIC_TK_ICON_THEME: &str = "icon_theme";

/// The largest size a theme lookup asks for. 08-ICONS.md §2.2's master export is 1024, but no
/// installed icon theme ships raster tiles anywhere near that; 256 is the common ceiling
/// (Adwaita, Papirus, hicolor all top out there) and `freedesktop_icons` falls back to the
/// closest size it has when a theme has nothing this large.
const MAX_SIZE: u16 = 256;

/// COSMIC's `icon_theme` (`com.system76.CosmicTk/v1/icon_theme`), else `Cosmic`, else
/// `hicolor`. The third step is `freedesktop_icons::lookup`'s own: every lookup falls back to the
/// `hicolor` theme whenever the chosen theme is not installed, so this
/// function only ever has to choose between the first two.
pub fn icon_theme(config: &Path) -> IconTheme {
    cosmic_tk_icon_theme(config).unwrap_or_else(|| IconTheme("Cosmic".to_owned()))
}

/// The path [`icon_theme`] reads under the config directory `config`:
/// `cosmic/com.system76.CosmicTk/v1/icon_theme` (`cosmic-config`'s own layout, read directly
/// rather than through the `cosmic-config` crate, which is not in the pinned dependency block).
pub fn cosmic_tk_icon_theme_path(config: &Path) -> PathBuf {
    config
        .join("cosmic")
        .join("com.system76.CosmicTk")
        .join("v1")
        .join(COSMIC_TK_ICON_THEME)
}

/// `cosmic-config` writes a RON string (`"Pop"`), one line, no trailing content; reading it is
/// just stripping the quotes. `None` when the file is absent, empty or unreadable (no theme has
/// been set, or COSMIC is not installed), which is exactly when the caller should fall back.
fn cosmic_tk_icon_theme(config: &Path) -> Option<IconTheme> {
    let raw = std::fs::read_to_string(cosmic_tk_icon_theme_path(config)).ok()?;
    let name = raw.trim().trim_matches('"');
    (!name.is_empty()).then(|| IconTheme(name.to_owned()))
}

/// `icon` (a name or an absolute path) as a file, the largest size up to [`MAX_SIZE`] px,
/// preferring an SVG over a PNG/XPM of the same or a smaller size. An absolute path is used
/// as-is, the way `IconSource::from_unknown` in `freedesktop_desktop_entry` itself treats one
/// (the entry's own word for where its icon is; not re-checked against the filesystem here,
/// same as every other field `parse_entry` takes on trust from the `.desktop` file); a name
/// goes through `freedesktop_icons` at `theme`, caching every lookup (`freedesktop_icons`'s own
/// process-wide cache).
pub fn resolve_icon(icon: &str, theme: &IconTheme) -> AppIcon {
    if icon.is_empty() {
        return AppIcon::None;
    }
    let as_path = Path::new(icon);
    if as_path.is_absolute() {
        return AppIcon::File(as_path.to_owned());
    }
    svg_lookup(icon, theme)
        .or_else(|| raster_lookup(icon, theme))
        .map_or(AppIcon::None, AppIcon::File)
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

    use super::{IconTheme, cosmic_tk_icon_theme_path, pixmap_dirs, resolve_icon};
    use crate::apps::index::AppIcon;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    // SVG-vs-raster preference against a real fixture theme directory is `tests/icons.rs` (an
    // integration test): `freedesktop_icons`' theme table is a process-wide lazy static seeded
    // from `$HOME`/XDG env once, and this workspace denies `unsafe_code` (CONVENTIONS §11), so
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
    fn resolve_icon_uses_an_absolute_path_as_is_without_checking_it_exists() {
        let temp = TempDir::new("icons-abs");
        let file = temp.path().join("app.png");
        fs::write(&file, b"png").unwrap_or_else(|e| panic!("{e}"));
        let theme = IconTheme("Fixture".to_owned());

        assert_eq!(
            resolve_icon(file.to_str().unwrap_or_else(|| panic!("utf8 path")), &theme),
            AppIcon::File(file)
        );
        assert_eq!(
            resolve_icon("/does/not/exist.png", &theme),
            AppIcon::File(PathBuf::from("/does/not/exist.png")),
            "an absolute Icon= is taken on trust, the same as every other field parse_entry \
             reads from the .desktop file"
        );
        assert_eq!(resolve_icon("", &theme), AppIcon::None);
    }

    #[test]
    fn cosmic_tk_icon_theme_path_ends_in_the_documented_layout() {
        let path = cosmic_tk_icon_theme_path(Path::new("/c"));
        assert!(
            path == Path::new("/c/cosmic/com.system76.CosmicTk/v1/icon_theme"),
            "{path:?}"
        );
    }

    #[test]
    fn pixmap_dirs_is_the_one_freedesktop_icons_itself_falls_back_to() {
        assert_eq!(pixmap_dirs(), vec![PathBuf::from("/usr/share/pixmaps")]);
    }
}
