//! Reading desktop entries from disk (effects: the filesystem).

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use freedesktop_desktop_entry::DesktopEntry;
use keycap_protocol::ProfileName;
use sill_launcher::AppId;
use sill_model::apps::{App, AppIcon, DbusActivation, DesktopActionEntry, Listing, Terminal};

use super::icon_roots::IconRoots;
use super::icons::{IconTheme, resolve_icon};

/// The desktop names this scan targets. sill runs as the desktop's own session
/// (`quire`), so `OnlyShowIn`/`NotShowIn` are matched against these constants rather than the
/// live `XDG_CURRENT_DESKTOP`, which keeps the result deterministic under a nested test
/// compositor too. An entry shown in either is shown here.
const CURRENT_DESKTOPS: [&str; 1] = ["quire"];

/// `$XDG_DATA_HOME/applications` then each `$XDG_DATA_DIRS/applications`, in precedence order
/// (`freedesktop_desktop_entry::default_paths`), with the app stores' own export directories
/// (Flatpak's, snap's) added at the end when they are not already on `XDG_DATA_DIRS` (a login
/// session that has sourced `/etc/profile.d/flatpak.sh` or snapd's profile script already has them
/// there, at the same low precedence; this is a safety net for one that has not). `data_home` is
/// where the user's own Flatpak installs.
pub fn applications_dirs(data_home: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = freedesktop_desktop_entry::default_paths().collect();
    for extra in store_export_dirs(data_home) {
        if !dirs.contains(&extra) {
            dirs.push(extra);
        }
    }
    dirs
}

/// The app stores' well-known export directories: Flatpak's user install, then its system one,
/// then snap's (Ubuntu's Firefox is a snap).
fn store_export_dirs(data_home: &Path) -> Vec<PathBuf> {
    vec![
        data_home.join("flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
        PathBuf::from("/var/lib/snapd/desktop/applications"),
    ]
}

/// Every entry under `dirs`, in precedence order, icons resolved in `theme` (an absolute `Icon=`
/// only inside `roots`), with the desktop's current locales. A directory that does not exist, or a `.desktop` file that
/// cannot be read or parsed, is skipped rather than failing the scan (`freedesktop_desktop_entry::Iter`
/// already skips a directory it cannot read).
pub fn scan(
    dirs: &[PathBuf],
    theme: &IconTheme,
    roots: &IconRoots,
    locales: &[String],
) -> Vec<App> {
    freedesktop_desktop_entry::Iter::new(dirs.iter().cloned())
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let id = entry_id(&path, dirs)?;
            parse_entry(&path, &id, &text, theme, roots, locales)
        })
        .collect()
}

/// `path`'s id relative to whichever of `dirs` contains it: `kde/foo.desktop` under an
/// applications directory is `kde-foo`. `None` when `path` is not actually under any of `dirs`
/// (should not happen for a path `Iter` itself yielded, but a scan never panics on it) or the
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

/// One `.desktop` file's text as an [`App`] with id derived from `path` relative to its
/// applications directory (`kde/foo.desktop` is `kde-foo`); `None` for a non-Application type,
/// an entry without `Name`/`Exec`, or one whose `TryExec` names a binary not found on `PATH`
/// (the Desktop Entry Specification's own words: such an entry "may be ignored" — it is not
/// really installed, so it should not occupy a dedup slot or ever be offered to launch).
pub fn parse_entry(
    path: &Path,
    id: &str,
    text: &str,
    theme: &IconTheme,
    roots: &IconRoots,
    locales: &[String],
) -> Option<App> {
    let entry = DesktopEntry::from_str(path, text, Some(locales)).ok()?;
    if !is_application(&entry) {
        return None;
    }
    let name = entry.name(locales)?.into_owned();
    let exec = entry.exec()?.to_owned();
    if entry.try_exec().is_some_and(|bin| !on_path(bin)) {
        return None;
    }

    Some(App {
        id: AppId(id.to_owned()),
        name,
        generic_name: owned(entry.generic_name(locales)),
        comment: owned(entry.comment(locales)),
        exec,
        icon: entry
            .icon()
            .map_or(AppIcon::None, |icon| resolve_icon(icon, theme, roots)),
        startup_wm_class: entry.startup_wm_class().map(str::to_owned),
        flatpak: entry.flatpak().map(str::to_owned),
        terminal: if entry.terminal() {
            Terminal::Yes
        } else {
            Terminal::No
        },
        activation: if entry.dbus_activatable() {
            DbusActivation::DBus
        } else {
            DbusActivation::Exec
        },
        actions: actions(&entry, locales),
        mime_types: owned_list(entry.mime_type()),
        categories: owned_list(entry.categories()),
        keyboard_profile: keyboard_profile(&entry, path),
        keywords: entry
            .keywords(locales)
            .unwrap_or_default()
            .into_iter()
            .map(Cow::into_owned)
            .filter(|k| !k.is_empty())
            .collect(),
        listing: if is_listed(&entry) {
            Listing::Shown
        } else {
            Listing::Hidden
        },
        path: path.to_owned(),
    })
}

fn owned(field: Option<Cow<'_, str>>) -> String {
    field.map(Cow::into_owned).unwrap_or_default()
}

/// A `;`-separated list field (`MimeType`, `Categories`, `Implements`): the spec's own
/// recommended form ends every list in a trailing `;`, which `str::split(';')` turns into a
/// spurious empty final element, so it is dropped here rather than by every caller.
fn owned_list(field: Option<Vec<&str>>) -> Vec<String> {
    field
        .unwrap_or_default()
        .into_iter()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The desktop entry key naming the keyboard profile an app wants.
const KEYBOARD_PROFILE_KEY: &str = "X-Keycap-Profile";

/// `X-Keycap-Profile`, parsed once here. A value that is not a profile name costs only this
/// key: it is logged and the entry is indexed without a profile, as every other bad field.
fn keyboard_profile(entry: &DesktopEntry, path: &Path) -> Option<ProfileName> {
    let value = entry.desktop_entry(KEYBOARD_PROFILE_KEY)?.trim();
    ProfileName::parse(value)
        .inspect_err(|error| {
            eprintln!(
                "sill: apps: {}: {KEYBOARD_PROFILE_KEY} {value:?} ignored: {error}",
                path.display()
            )
        })
        .ok()
}

/// `Type=Application`, or no `Type` at all (a handful of real-world entries omit it; treating
/// that as valid rather than refusing the whole file matches how `freedesktop_desktop_entry`'s
/// own accessors already treat every other missing field).
fn is_application(entry: &DesktopEntry) -> bool {
    entry.type_().is_none_or(|kind| kind == "Application")
}

/// Whether `try_exec`, an absolute path or a bare command name, resolves to a file: absolute
/// paths are checked directly, a bare name is searched on `$PATH` (CONVENTIONS "substrings are
/// not tokens" does not apply here — this is a filesystem existence check, not a string match).
fn on_path(try_exec: &str) -> bool {
    let candidate = Path::new(try_exec);
    if candidate.is_absolute() {
        return candidate.is_file();
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(try_exec).is_file()))
        .unwrap_or(false)
}

/// `NoDisplay`, `Hidden` and an `OnlyShowIn`/`NotShowIn` mismatch against [`CURRENT_DESKTOPS`]
/// all fold into the same [`Listing::Hidden`] (the frozen `App::listing`'s own doc: "hidden
/// entries still match windows, never list"); this scan never excludes an entry outright for
/// any of them; the whole record is still yielded and dedupes, so a `Hidden=true` entry in a
/// higher-precedence directory still shadows a lower one for `match_app`.
fn is_listed(entry: &DesktopEntry) -> bool {
    if entry.no_display() || entry.hidden() {
        return false;
    }
    if let Some(only) = entry.only_show_in()
        && !only.iter().any(|name| is_ours(name))
    {
        return false;
    }
    if let Some(not) = entry.not_show_in()
        && not.iter().any(|name| is_ours(name))
    {
        return false;
    }
    true
}

/// Whether `name` (an `OnlyShowIn`/`NotShowIn` item) names a desktop this scan targets.
fn is_ours(name: &str) -> bool {
    CURRENT_DESKTOPS.contains(&name)
}

fn actions(entry: &DesktopEntry, locales: &[String]) -> Vec<DesktopActionEntry> {
    entry
        .actions()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|action| {
            let name = entry.action_name(action, locales)?.into_owned();
            let exec = entry.action_exec(action)?.to_owned();
            Some(DesktopActionEntry {
                id: action.to_owned(),
                name,
                exec,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::icon_roots::IconRoots;
    use super::super::icons::IconTheme;
    use super::{entry_id, is_application, is_listed, on_path, parse_entry, store_export_dirs};
    use freedesktop_desktop_entry::DesktopEntry;
    use keycap_protocol::ProfileName;
    use sill_model::apps::Listing;
    use std::path::{Path, PathBuf};

    fn entry(text: &str) -> DesktopEntry {
        DesktopEntry::from_str(Path::new("/tmp/t.desktop"), text, Some(&[] as &[&str]))
            .unwrap_or_else(|e| panic!("{e}"))
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
        let dirs = store_export_dirs(Path::new("/home/a/.local/share"));
        let want = [
            "/home/a/.local/share/flatpak/exports/share/applications",
            "/var/lib/flatpak/exports/share/applications",
            "/var/lib/snapd/desktop/applications",
        ];
        assert_eq!(dirs, want.map(PathBuf::from));
    }

    #[test]
    fn is_application_accepts_application_or_no_type_and_refuses_the_rest() {
        let cases: &[(&str, &str, bool)] = &[
            ("explicit", "[Desktop Entry]\nType=Application\n", true),
            ("missing", "[Desktop Entry]\nName=X\n", true),
            ("link", "[Desktop Entry]\nType=Link\nURL=x\n", false),
            ("directory", "[Desktop Entry]\nType=Directory\n", false),
        ];
        for (name, text, want) in cases {
            assert_eq!(is_application(&entry(text)), *want, "{name}");
        }
    }

    #[test]
    fn is_listed_hides_no_display_hidden_and_a_desktop_mismatch() {
        let cases: &[(&str, &str, bool)] = &[
            ("plain", "[Desktop Entry]\nType=Application\n", true),
            (
                "no display",
                "[Desktop Entry]\nType=Application\nNoDisplay=true\n",
                false,
            ),
            (
                "hidden",
                "[Desktop Entry]\nType=Application\nHidden=true\n",
                false,
            ),
            (
                "only quire",
                "[Desktop Entry]\nType=Application\nOnlyShowIn=quire;\n",
                true,
            ),
            (
                "quire or gnome",
                "[Desktop Entry]\nType=Application\nOnlyShowIn=GNOME;quire;\n",
                true,
            ),
            (
                "not quire",
                "[Desktop Entry]\nType=Application\nNotShowIn=quire;\n",
                false,
            ),
            (
                "only gnome",
                "[Desktop Entry]\nType=Application\nOnlyShowIn=GNOME;\n",
                false,
            ),
            (
                "not gnome",
                "[Desktop Entry]\nType=Application\nNotShowIn=GNOME;\n",
                true,
            ),
            (
                "quire is not a substring match away",
                "[Desktop Entry]\nType=Application\nOnlyShowIn=Xquire;\n",
                false,
            ),
        ];
        for (name, text, want) in cases {
            assert_eq!(is_listed(&entry(text)), *want, "{name}");
        }
    }

    #[test]
    fn on_path_checks_absolute_files_directly_and_bare_names_on_path() {
        // `/bin/sh` and a bare `sh` on `$PATH` exist on every machine that can run this test
        // suite at all (`std::process::Command` itself relies on the same).
        assert!(on_path("/bin/sh"));
        assert!(on_path("sh"));
        assert!(!on_path("/no/such/binary-xyz"));
        assert!(!on_path("no-such-binary-xyz"));
    }

    #[test]
    fn parse_entry_refuses_a_try_exec_that_is_not_installed() {
        let text =
            "[Desktop Entry]\nType=Application\nName=X\nExec=x\nTryExec=/no/such/binary-xyz\n";
        let theme = IconTheme("hicolor".to_owned());
        assert!(
            parse_entry(
                Path::new("/a/x.desktop"),
                "x",
                text,
                &theme,
                &IconRoots::default(),
                &[]
            )
            .is_none()
        );
    }

    #[test]
    fn parse_entry_refuses_without_name_or_exec() {
        let theme = IconTheme("hicolor".to_owned());
        let no_name = "[Desktop Entry]\nType=Application\nExec=x\n";
        let no_exec = "[Desktop Entry]\nType=Application\nName=X\n";
        assert!(
            parse_entry(
                Path::new("/a/x.desktop"),
                "x",
                no_name,
                &theme,
                &IconRoots::default(),
                &[]
            )
            .is_none()
        );
        assert!(
            parse_entry(
                Path::new("/a/x.desktop"),
                "x",
                no_exec,
                &theme,
                &IconRoots::default(),
                &[]
            )
            .is_none()
        );
    }

    #[test]
    fn parse_entry_marks_a_hidden_entry_shown_as_listing_hidden_but_still_yields_it() {
        let text = "[Desktop Entry]\nType=Application\nName=X\nExec=x\nNoDisplay=true\n";
        let theme = IconTheme("hicolor".to_owned());
        let app = parse_entry(
            Path::new("/a/x.desktop"),
            "x",
            text,
            &theme,
            &IconRoots::default(),
            &[],
        )
        .unwrap_or_else(|| panic!("expected an App"));
        assert_eq!(app.listing, Listing::Hidden);
        assert_eq!(app.name, "X");
    }

    #[test]
    fn parse_entry_reads_categories_and_a_declared_keyboard_profile() {
        const HEAD: &str = "[Desktop Entry]\nType=Application\nName=X\nExec=x\n";
        const CASES: &[(&str, &str, &[&str], Option<&str>)] = &[
            ("neither", "", &[], None),
            (
                "categories with the trailing semicolon",
                "Categories=System;TerminalEmulator;\n",
                &["System", "TerminalEmulator"],
                None,
            ),
            (
                "a declared profile",
                "X-Keycap-Profile=terminal\n",
                &[],
                Some("terminal"),
            ),
            (
                "a profile name is trimmed",
                "X-Keycap-Profile= game-1 \n",
                &[],
                Some("game-1"),
            ),
            (
                "an invalid profile is ignored",
                "X-Keycap-Profile=Not A Name\n",
                &[],
                None,
            ),
            (
                "an empty profile is ignored",
                "X-Keycap-Profile=\n",
                &[],
                None,
            ),
        ];
        let theme = IconTheme("hicolor".to_owned());
        for &(name, extra, categories, profile) in CASES {
            let text = format!("{HEAD}{extra}");
            let app = parse_entry(
                Path::new("/a/x.desktop"),
                "x",
                &text,
                &theme,
                &IconRoots::default(),
                &[],
            )
            .unwrap_or_else(|| panic!("{name}: expected an App"));
            assert_eq!(app.categories, categories, "{name}");
            assert_eq!(
                app.keyboard_profile,
                profile.map(|p| ProfileName::parse(p).unwrap_or_else(|e| panic!("{e}"))),
                "{name}"
            );
        }
    }
}
