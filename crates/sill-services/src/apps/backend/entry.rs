//! One `.desktop` file's text as an [`App`].

use std::borrow::Cow;
use std::path::Path;

use ds_helpers::{ApplicationEntry, DesktopFile, DesktopGroup, EntryLocale};
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

/// The desktop entry key naming the keyboard profile an app wants.
const KEYBOARD_PROFILE_KEY: &str = "X-Keycap-Profile";

/// One loaded `.desktop` file as an [`App`], its id the entry's desktop id without the suffix
/// (`kde/foo.desktop` is `kde-foo`, [`ds_helpers::DesktopId`]'s rule); `None` for a
/// non-Application type, an entry without `Name`/`Exec`, or one whose
/// `TryExec` names a binary not found on `PATH` (the Desktop Entry Specification's own words:
/// such an entry "may be ignored" — it is not really installed, so it should not occupy a dedup
/// slot or ever be offered to launch). Translated words are picked for `locales`, best first.
pub fn parse_entry(
    loaded: &ApplicationEntry,
    theme: &IconTheme,
    roots: &IconRoots,
    locales: &[EntryLocale],
) -> Option<App> {
    let file = &loaded.file;
    let entry = file.entry()?;
    if !is_application(entry) {
        return None;
    }
    let name = entry.localized("Name", locales)?.into_owned();
    let exec = entry.string("Exec")?.into_owned();
    if entry.string("TryExec").is_some_and(|bin| !on_path(&bin)) {
        return None;
    }

    let identity = loaded.identity()?;
    Some(App {
        id: AppId(loaded.id.stem().to_owned()),
        name,
        generic_name: words(entry, "GenericName", locales),
        comment: words(entry, "Comment", locales),
        exec,
        icon: entry
            .string("Icon")
            .map_or(AppIcon::None, |icon| resolve_icon(&icon, theme, roots)),
        startup_wm_class: loaded.keys.startup_wm_class.clone(),
        flatpak: loaded.keys.flatpak.clone(),
        bus_names: identity.bus_names,
        intents_app: identity.intents_app,
        terminal: if entry.is_true("Terminal") {
            Terminal::Yes
        } else {
            Terminal::No
        },
        activation: if entry.is_true("DBusActivatable") {
            DbusActivation::DBus
        } else {
            DbusActivation::Exec
        },
        actions: actions(file, entry, locales),
        mime_types: entry.list("MimeType"),
        categories: entry.list("Categories"),
        keyboard_profile: keyboard_profile(entry, &loaded.path),
        keywords: entry.localized_list("Keywords", locales),
        listing: if is_listed(entry) {
            Listing::Shown
        } else {
            Listing::Hidden
        },
        path: loaded.path.clone(),
    })
}

/// The translated text of `key`, empty when the entry has none.
fn words(entry: &DesktopGroup, key: &str, locales: &[EntryLocale]) -> String {
    entry
        .localized(key, locales)
        .map(Cow::into_owned)
        .unwrap_or_default()
}

/// `X-Keycap-Profile`, parsed once here. A value that is not a profile name costs only this
/// key: it is logged and the entry is indexed without a profile, as every other bad field.
fn keyboard_profile(entry: &DesktopGroup, path: &Path) -> Option<ProfileName> {
    let value = entry.string(KEYBOARD_PROFILE_KEY)?;
    let value = value.trim();
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
/// that as valid rather than refusing the whole file is how every other missing field is read).
fn is_application(entry: &DesktopGroup) -> bool {
    entry
        .string("Type")
        .is_none_or(|kind| kind == "Application")
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
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(try_exec).is_file()))
}

/// `NoDisplay`, `Hidden` and an `OnlyShowIn`/`NotShowIn` mismatch against [`CURRENT_DESKTOPS`]
/// all fold into the same [`Listing::Hidden`] (the frozen `App::listing`'s own doc: "hidden
/// entries still match windows, never list"); this scan never excludes an entry outright for
/// any of them; the whole record is still yielded and dedupes, so a `Hidden=true` entry in a
/// higher-precedence directory still shadows a lower one for `match_app`.
fn is_listed(entry: &DesktopGroup) -> bool {
    if entry.is_true("NoDisplay") || entry.is_true("Hidden") {
        return false;
    }
    let only = entry.list("OnlyShowIn");
    if !only.is_empty() && !only.iter().any(|name| is_ours(name)) {
        return false;
    }
    !entry.list("NotShowIn").iter().any(|name| is_ours(name))
}

/// Whether `name` (an `OnlyShowIn`/`NotShowIn` item) names a desktop this scan targets.
fn is_ours(name: &str) -> bool {
    CURRENT_DESKTOPS.contains(&name)
}

/// The entry's desktop actions: each id in `Actions` whose `[Desktop Action id]` group has a
/// `Name` and an `Exec`, in the order the entry lists them.
fn actions(
    file: &DesktopFile,
    entry: &DesktopGroup,
    locales: &[EntryLocale],
) -> Vec<DesktopActionEntry> {
    entry
        .list("Actions")
        .into_iter()
        .filter_map(|id| {
            let group = file.group(&format!("Desktop Action {id}"))?;
            let name = group.localized("Name", locales)?.into_owned();
            let exec = group.string("Exec")?.into_owned();
            Some(DesktopActionEntry { id, name, exec })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::icon_roots::IconRoots;
    use super::super::icons::IconTheme;
    use super::{is_application, is_listed, on_path, parse_entry};
    use ds_helpers::{
        AppKeys, ApplicationEntry, DESKTOP_ENTRY_CAP, DesktopFile, DesktopId, EntryLocale,
    };
    use keycap_protocol::ProfileName;
    use sill_model::apps::{App, Listing};
    use std::path::Path;

    fn file(text: &str) -> DesktopFile {
        DesktopFile::parse(text, DESKTOP_ENTRY_CAP).unwrap_or_else(|e| panic!("{e}"))
    }

    /// `text` as an app for `locales` (locale texts such as `de_AT.UTF-8`), loaded as the scan
    /// would load `/a/x.desktop`.
    fn app_in(text: &str, locales: &[&str]) -> Option<App> {
        let locales: Vec<EntryLocale> = locales
            .iter()
            .filter_map(|l| EntryLocale::parse(l))
            .collect();
        let loaded = ApplicationEntry {
            path: Path::new("/a/x.desktop").to_owned(),
            id: DesktopId::parse("x")?,
            keys: AppKeys::default(),
            file: DesktopFile::parse(text, DESKTOP_ENTRY_CAP).ok()?,
        };
        parse_entry(
            &loaded,
            &IconTheme("hicolor".to_owned()),
            &IconRoots::default(),
            &locales,
        )
    }

    fn app(text: &str) -> Option<App> {
        app_in(text, &[])
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
            let file = file(text);
            let entry = file.entry().unwrap_or_else(|| panic!("{name}: no entry"));
            assert_eq!(is_application(entry), *want, "{name}");
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
            let file = file(text);
            let entry = file.entry().unwrap_or_else(|| panic!("{name}: no entry"));
            assert_eq!(is_listed(entry), *want, "{name}");
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
        assert!(app(text).is_none());
    }

    #[test]
    fn parse_entry_refuses_without_name_or_exec() {
        let no_name = "[Desktop Entry]\nType=Application\nExec=x\n";
        let no_exec = "[Desktop Entry]\nType=Application\nName=X\n";
        for (name, text) in [("no name", no_name), ("no exec", no_exec)] {
            assert!(app(text).is_none(), "{name}");
        }
    }

    #[test]
    fn parse_entry_marks_a_hidden_entry_shown_as_listing_hidden_but_still_yields_it() {
        let text = "[Desktop Entry]\nType=Application\nName=X\nExec=x\nNoDisplay=true\n";
        let app = app(text).unwrap_or_else(|| panic!("expected an App"));
        assert_eq!(app.listing, Listing::Hidden);
        assert_eq!(app.name, "X");
    }

    #[test]
    fn parse_entry_picks_translated_words_for_the_session_locale() {
        const TEXT: &str = "[Desktop Entry]\nName=Files\nName[de]=Dateien\nName[de_AT]=Dateien AT\n\
            Comment=Browse\nComment[de]=Stöbern\nExec=x\nKeywords=a;b;\nKeywords[de]=c;d;\n";
        // `(locales, name, comment, keywords)`: the first locale a key is translated into wins,
        // the language alone answers a country, and the default answers the rest.
        const CASES: &[(&[&str], &str, &str, &[&str])] = &[
            (&[], "Files", "Browse", &["a", "b"]),
            (&["C"], "Files", "Browse", &["a", "b"]),
            (&["de_AT.UTF-8"], "Dateien AT", "Stöbern", &["c", "d"]),
            (&["de_CH"], "Dateien", "Stöbern", &["c", "d"]),
            (&["fr", "de"], "Dateien", "Stöbern", &["c", "d"]),
            (&["fr"], "Files", "Browse", &["a", "b"]),
        ];
        for &(locales, name, comment, keywords) in CASES {
            let app = app_in(TEXT, locales).unwrap_or_else(|| panic!("{locales:?}: no App"));
            assert_eq!(app.name, name, "{locales:?}");
            assert_eq!(app.comment, comment, "{locales:?}");
            assert_eq!(app.keywords, keywords, "{locales:?}");
        }
    }

    #[test]
    fn parse_entry_reads_the_actions_an_entry_lists_with_a_name_and_an_exec() {
        const TEXT: &str = "[Desktop Entry]\nName=X\nExec=x\n\
            Actions=new;private;unnamed;noexec;missing;\n\
            [Desktop Action new]\nName=New Window\nName[de]=Neues Fenster\nExec=x --new\n\
            [Desktop Action private]\nName=Private\nExec=x --private\n\
            [Desktop Action unnamed]\nExec=x --unnamed\n\
            [Desktop Action noexec]\nName=No Exec\n\
            [Desktop Action unlisted]\nName=Unlisted\nExec=x --unlisted\n";
        /// `(locales, [(id, name, exec)])`.
        type Case = (
            &'static [&'static str],
            &'static [(&'static str, &'static str, &'static str)],
        );
        const CASES: &[Case] = &[
            (
                &[],
                &[
                    ("new", "New Window", "x --new"),
                    ("private", "Private", "x --private"),
                ],
            ),
            (
                &["de_DE"],
                &[
                    ("new", "Neues Fenster", "x --new"),
                    ("private", "Private", "x --private"),
                ],
            ),
        ];
        for &(locales, want) in CASES {
            let app = app_in(TEXT, locales).unwrap_or_else(|| panic!("{locales:?}: no App"));
            let got: Vec<(&str, &str, &str)> = app
                .actions
                .iter()
                .map(|a| (a.id.as_str(), a.name.as_str(), a.exec.as_str()))
                .collect();
            assert_eq!(got, want, "{locales:?}");
        }
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
        for &(name, extra, categories, profile) in CASES {
            let text = format!("{HEAD}{extra}");
            let app = app(&text).unwrap_or_else(|| panic!("{name}: expected an App"));
            assert_eq!(app.categories, categories, "{name}");
            assert_eq!(
                app.keyboard_profile,
                profile.map(|p| ProfileName::parse(p).unwrap_or_else(|e| panic!("{e}"))),
                "{name}"
            );
        }
    }

    #[test]
    fn parse_entry_carries_the_names_the_entry_declares() {
        const TEXT: &str = "[Desktop Entry]\nName=Mail\nExec=mailo\n\
            X-Quire-BusNames=io.github.PoHsuanLai.mailo;org.quire.Mail;\n\
            X-Quire-IntentsApp=org.quire.Mail\n";
        let app = app(TEXT).unwrap_or_else(|| panic!("expected an App"));
        assert_eq!(
            app.bus_names,
            ["io.github.PoHsuanLai.mailo", "org.quire.Mail"]
        );
        assert_eq!(app.intents_app.as_deref(), Some("org.quire.Mail"));
        let bare =
            self::app("[Desktop Entry]\nName=X\nExec=x\n").unwrap_or_else(|| panic!("no App"));
        assert!(bare.bus_names.is_empty() && bare.intents_app.is_none());
    }
}
