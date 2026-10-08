//! `calendar.sources`, parsed, and what `auto` finds: pure over the paths the caller read.

use std::path::{Path, PathBuf};

/// One `.ics` place.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IcsPlace {
    /// One `.ics` file: one calendar.
    File(PathBuf),
    /// A directory: a vdir collection itself, or a parent of collections (provider doc).
    Dir(PathBuf),
    /// A directory of directories of collections: porter's `vdir/<account>/<collection>`. Read
    /// afresh each time, so an account or a collection that appears later is found, and watched
    /// as one directory.
    Vdir(PathBuf),
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

/// Where `auto` looks, in order: `$XDG_DATA_HOME/sill/calendars` (a directory to drop `.ics`
/// files into), `$XDG_DATA_HOME/porter/vdir` (what porter's syncd mirrors from the accounts
/// the person granted it: `<account>/<collection>/*.ics`) and `~/.calendars` (vdirsyncer and
/// khal's convention). Candidates only: the caller keeps those that exist (`provider_for`).
pub fn auto_places(env: &AutoEnv) -> Vec<IcsPlace> {
    vec![
        IcsPlace::Dir(env.data_home.join("sill").join("calendars")),
        IcsPlace::Vdir(env.data_home.join("porter").join("vdir")),
        IcsPlace::Dir(env.home.join(".calendars")),
    ]
}

/// Whether a vdir collection directory is a task list (VTODO), not a calendar: porter names it
/// `<collection>-tasks`. The calendar view skips it.
pub fn is_task_list(dir: &Path) -> bool {
    dir.file_name()
        .is_some_and(|name| name.to_string_lossy().ends_with("-tasks"))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{AutoEnv, EventSources, IcsPlace, auto_places, is_task_list, parse_sources};

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
    fn auto_looks_at_sill_porter_and_vdirsyncer() {
        let env = AutoEnv {
            home: PathBuf::from(HOME),
            data_home: PathBuf::from("/home/ada/.local/share"),
        };
        assert_eq!(
            auto_places(&env),
            vec![
                IcsPlace::Dir(PathBuf::from("/home/ada/.local/share/sill/calendars")),
                IcsPlace::Vdir(PathBuf::from("/home/ada/.local/share/porter/vdir")),
                IcsPlace::Dir(PathBuf::from("/home/ada/.calendars")),
            ]
        );
    }

    #[test]
    fn a_collection_ending_in_tasks_is_a_task_list() {
        assert!(is_task_list(Path::new("/v/google/personal-tasks")));
        assert!(!is_task_list(Path::new("/v/google/personal")));
        assert!(!is_task_list(Path::new("/v/google/tasks-and-more")));
    }
}
