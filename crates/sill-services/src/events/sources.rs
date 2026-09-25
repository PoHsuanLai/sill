//! `calendar.sources`, parsed, and what `auto` finds: pure over the paths the caller read.

use std::path::{Path, PathBuf};

/// One `.ics` place.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IcsPlace {
    /// One `.ics` file: one calendar.
    File(PathBuf),
    /// A directory: a vdir collection itself, or a parent of collections (provider doc).
    Dir(PathBuf),
}

/// What `calendar.sources` says.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum EventSources {
    /// Read nothing.
    #[default]
    Off,
    /// The places [`auto_places`] finds.
    Auto,
    /// These places, in the setting's order.
    Places(Vec<IcsPlace>),
}

/// What [`auto_places`] looks at, read by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoEnv {
    /// `$HOME`.
    pub home: PathBuf,
    /// `$XDG_DATA_HOME` (else `~/.local/share`).
    pub data_home: PathBuf,
    /// The text of `$XDG_CONFIG_HOME/akonadi_ical_resource_0rc`, when there is one.
    pub akonadi_rc: Option<String>,
}

/// `calendar.sources`: `auto` and `off` (any case, trimmed; empty is `off`), else `;`-separated
/// paths, each trimmed, `~/` expanded against `home`, a path ending in `.ics` a file and any
/// other a directory; empty entries skipped.
pub fn parse_sources(text: &str, home: &Path) -> EventSources {
    let trimmed = text.trim();
    match trimmed.to_ascii_lowercase().as_str() {
        "auto" => EventSources::Auto,
        "off" | "" => EventSources::Off,
        _ => {
            let places: Vec<IcsPlace> = trimmed
                .split(';')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(|entry| place_of(expand_home(entry, home)))
                .collect();
            match places.is_empty() {
                true => EventSources::Off,
                false => EventSources::Places(places),
            }
        }
    }
}

/// `~` and `~/…` against `home`; anything else as written.
fn expand_home(entry: &str, home: &Path) -> PathBuf {
    match entry.strip_prefix('~') {
        Some("") => home.to_path_buf(),
        Some(rest) if rest.starts_with('/') => home.join(rest.trim_start_matches('/')),
        _ => PathBuf::from(entry),
    }
}

/// A path ending in `.ics` (any case) is a file; any other a directory.
fn place_of(path: PathBuf) -> IcsPlace {
    let is_ics = path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ics"));
    match is_ics {
        true => IcsPlace::File(path),
        false => IcsPlace::Dir(path),
    }
}

/// The file KOrganizer's calendar lives in, from Akonadi's iCal resource config
/// (`[General]` `Path[$e]=$HOME/.local/share/apps/korganizer/std.ics`): `$HOME` expanded against
/// `home`; `None` without a `Path` key.
pub fn akonadi_ical_path(rc: &str, home: &Path) -> Option<PathBuf> {
    let home = home.to_string_lossy();
    rc.lines()
        .map(str::trim)
        .filter_map(|line| line.split_once('='))
        .find(|(key, _)| {
            let key = key.trim();
            key == "Path" || key.starts_with("Path[")
        })
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| PathBuf::from(value.replace("${HOME}", &home).replace("$HOME", &home)))
}

/// Where `auto` looks, in order: KOrganizer's file ([`akonadi_ical_path`]), when the rc names
/// one; `$XDG_DATA_HOME/sill/calendars` (a directory to drop `.ics` files into); and
/// `~/.calendars` (vdirsyncer and khal's convention). Candidates only: the caller keeps those
/// that exist (`provider_for`).
pub fn auto_places(env: &AutoEnv) -> Vec<IcsPlace> {
    let korganizer = env
        .akonadi_rc
        .as_deref()
        .and_then(|rc| akonadi_ical_path(rc, &env.home))
        .map(IcsPlace::File);
    korganizer
        .into_iter()
        .chain([
            IcsPlace::Dir(env.data_home.join("sill").join("calendars")),
            IcsPlace::Dir(env.home.join(".calendars")),
        ])
        .collect()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{AutoEnv, EventSources, IcsPlace, akonadi_ical_path, auto_places, parse_sources};

    const HOME: &str = "/home/ada";

    #[test]
    fn the_setting_parses_into_places() {
        let home = Path::new(HOME);
        let file = |p: &str| IcsPlace::File(PathBuf::from(p));
        let dir = |p: &str| IcsPlace::Dir(PathBuf::from(p));
        let cases = [
            ("auto", EventSources::Auto),
            (" AUTO ", EventSources::Auto),
            ("off", EventSources::Off),
            ("", EventSources::Off),
            (
                "~/cal/work.ics; /srv/dav ;",
                EventSources::Places(vec![file("/home/ada/cal/work.ics"), dir("/srv/dav")]),
            ),
        ];
        for (text, want) in cases {
            assert_eq!(parse_sources(text, home), want, "{text:?}");
        }
    }

    #[test]
    fn akonadis_ical_resource_names_korganizers_file() {
        let home = Path::new(HOME);
        let cases = [
            (
                "[General]\nPath[$e]=$HOME/.local/share/apps/korganizer/std.ics\n",
                Some(PathBuf::from(
                    "/home/ada/.local/share/apps/korganizer/std.ics",
                )),
            ),
            (
                "[General]\nPath=/srv/cal.ics\nReadOnly=false\n",
                Some(PathBuf::from("/srv/cal.ics")),
            ),
            ("[General]\nReadOnly=true\n", None),
        ];
        for (rc, want) in cases {
            assert_eq!(akonadi_ical_path(rc, home), want, "{rc:?}");
        }
    }

    #[test]
    fn auto_looks_at_korganizer_sill_and_vdirsyncer() {
        let korganizer = PathBuf::from("/home/ada/.local/share/apps/korganizer/std.ics");
        let sill = IcsPlace::Dir(PathBuf::from("/home/ada/.local/share/sill/calendars"));
        let vdir = IcsPlace::Dir(PathBuf::from("/home/ada/.calendars"));
        let env = AutoEnv {
            home: PathBuf::from(HOME),
            data_home: PathBuf::from("/home/ada/.local/share"),
            akonadi_rc: Some(format!("[General]\nPath={}\n", korganizer.display())),
        };
        assert_eq!(
            auto_places(&env),
            vec![IcsPlace::File(korganizer), sill.clone(), vdir.clone()]
        );
        let bare = AutoEnv {
            akonadi_rc: None,
            ..env
        };
        assert_eq!(auto_places(&bare), vec![sill, vdir]);
    }
}
