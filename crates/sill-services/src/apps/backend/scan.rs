//! Reading desktop entries from disk (effects: the filesystem).

use std::path::{Path, PathBuf};

use ds_helpers::{DESKTOP_ENTRY_CAP, EntryLocale, read_text_capped};
use sill_model::apps::App;

use super::entry::parse_entry;
use super::icon_roots::IconRoots;
use super::icons::IconTheme;
use super::walk::desktop_files;

/// The folder of desktop entries inside a data directory.
const APPLICATIONS: &str = "applications";

/// `data_home/applications` then each of `data_dirs`' (`$XDG_DATA_DIRS`), in precedence order,
/// with the app stores' own export directories (Flatpak's, snap's) added at the end when they are
/// not already there (a login session that has sourced `/etc/profile.d/flatpak.sh` or snapd's
/// profile script already has them on `XDG_DATA_DIRS`, at the same low precedence; this is a
/// safety net for one that has not). `data_home` is where the user's own Flatpak installs;
/// `system_stores` are the system stores' folders ([`Dirs::store_exports`](crate::env::Dirs)).
pub fn applications_dirs(
    data_home: &Path,
    data_dirs: &[PathBuf],
    system_stores: &[PathBuf],
) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::iter::once(data_home)
        .chain(data_dirs.iter().map(PathBuf::as_path))
        .map(|dir| dir.join(APPLICATIONS))
        .collect();
    for extra in store_export_dirs(data_home, system_stores) {
        if !dirs.contains(&extra) {
            dirs.push(extra);
        }
    }
    dirs
}

/// The app stores' export directories: Flatpak's user install, then the system stores'.
fn store_export_dirs(data_home: &Path, system_stores: &[PathBuf]) -> Vec<PathBuf> {
    std::iter::once(data_home.join("flatpak/exports/share/applications"))
        .chain(system_stores.iter().cloned())
        .collect()
}

/// Every entry under `dirs`, in precedence order, icons resolved in `theme` (an absolute `Icon=`
/// only inside `roots`), with the desktop's current locales. A directory that does not exist, or a
/// `.desktop` file that cannot be read (or is over [`DESKTOP_ENTRY_CAP`], or is not text), is
/// skipped rather than failing the scan.
pub fn scan(
    dirs: &[PathBuf],
    theme: &IconTheme,
    roots: &IconRoots,
    locales: &[EntryLocale],
) -> Vec<App> {
    desktop_files(dirs)
        .into_iter()
        .filter_map(|path| {
            let text = read_text_capped(&path, DESKTOP_ENTRY_CAP).ok()?;
            let id = entry_id(&path, dirs)?;
            parse_entry(&path, &id, &text, theme, roots, locales)
        })
        .collect()
}

/// `path`'s id relative to whichever of `dirs` contains it: `kde/foo.desktop` under an
/// applications directory is `kde-foo`. `None` when `path` is not actually under any of `dirs`
/// (should not happen for a path the walk itself yielded, but a scan never panics on it) or the
/// relative path is empty.
fn entry_id(path: &Path, dirs: &[PathBuf]) -> Option<String> {
    let relative = dirs.iter().find_map(|dir| {
        path.strip_prefix(dir).ok().or_else(|| {
            let canonical = dir.canonicalize().ok()?;
            path.strip_prefix(&canonical).ok()
        })
    })?;
    let relative = relative.with_extension("");
    let id = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("-");
    (!id.is_empty()).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::{applications_dirs, entry_id, store_export_dirs};
    use std::path::{Path, PathBuf};

    fn system_stores() -> [PathBuf; 2] {
        [
            PathBuf::from("/var/lib/flatpak/exports/share/applications"),
            PathBuf::from("/var/lib/snapd/desktop/applications"),
        ]
    }

    #[test]
    fn entry_id_is_relative_to_its_own_applications_dir_with_slashes_dashed() {
        let dirs = [PathBuf::from("/usr/share/applications")];
        let cases: &[(&str, Option<&str>)] = &[
            ("/usr/share/applications/foot.desktop", Some("foot")),
            ("/usr/share/applications/kde/foo.desktop", Some("kde-foo")),
            ("/other/place/foot.desktop", None),
        ];
        for (path, want) in cases {
            assert_eq!(
                entry_id(Path::new(path), &dirs),
                want.map(str::to_owned),
                "{path}"
            );
        }
    }

    #[test]
    fn store_export_dirs_name_flatpak_user_and_system_then_snap() {
        let dirs = store_export_dirs(Path::new("/home/a/.local/share"), &system_stores());
        let want = [
            "/home/a/.local/share/flatpak/exports/share/applications",
            "/var/lib/flatpak/exports/share/applications",
            "/var/lib/snapd/desktop/applications",
        ];
        assert_eq!(dirs, want.map(PathBuf::from));
    }

    #[test]
    fn applications_dirs_run_data_home_then_data_dirs_then_the_stores_not_already_there() {
        let home = Path::new("/home/a/.local/share");
        let data_dirs = [
            PathBuf::from("/usr/local/share"),
            PathBuf::from("/usr/share"),
        ];
        let plain = applications_dirs(home, &data_dirs, &system_stores());
        let want = [
            "/home/a/.local/share/applications",
            "/usr/local/share/applications",
            "/usr/share/applications",
            "/home/a/.local/share/flatpak/exports/share/applications",
            "/var/lib/flatpak/exports/share/applications",
            "/var/lib/snapd/desktop/applications",
        ];
        assert_eq!(plain, want.map(PathBuf::from));

        // A session that already lists the system Flatpak folder keeps it where it put it.
        let listed = [
            PathBuf::from("/var/lib/flatpak/exports/share"),
            PathBuf::from("/usr/share"),
        ];
        let got = applications_dirs(home, &listed, &system_stores());
        let flatpak = PathBuf::from("/var/lib/flatpak/exports/share/applications");
        assert_eq!(got.iter().filter(|dir| **dir == flatpak).count(), 1);
        assert_eq!(got.get(1), Some(&flatpak));
    }
}
