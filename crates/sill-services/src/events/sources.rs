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
    let _ = (text, home);
    todo!("M10 fill: a-calendar")
}

/// The file KOrganizer's calendar lives in, from Akonadi's iCal resource config
/// (`[General]` `Path[$e]=$HOME/.local/share/apps/korganizer/std.ics`): `$HOME` expanded against
/// `home`; `None` without a `Path` key.
pub fn akonadi_ical_path(rc: &str, home: &Path) -> Option<PathBuf> {
    let _ = (rc, home);
    todo!("M10 fill: a-calendar")
}

/// Where `auto` looks, in order: KOrganizer's file ([`akonadi_ical_path`]), when the rc names
/// one; `$XDG_DATA_HOME/sill/calendars` (a directory to drop `.ics` files into); and
/// `~/.calendars` (vdirsyncer and khal's convention). Candidates only: the caller keeps those
/// that exist (`provider_for`).
pub fn auto_places(env: &AutoEnv) -> Vec<IcsPlace> {
    let _ = env;
    todo!("M10 fill: a-calendar")
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{AutoEnv, EventSources, IcsPlace, akonadi_ical_path, auto_places, parse_sources};

    const HOME: &str = "/home/ada";

    #[test]
    #[ignore = "M10 fill: a-calendar"]
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
    #[ignore = "M10 fill: a-calendar"]
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
    #[ignore = "M10 fill: a-calendar"]
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
