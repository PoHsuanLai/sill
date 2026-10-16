//! The person's directories, read from the process once ([`Dirs::from_process`]) or set by a
//! test ([`Dirs::under`]), so no service reads `HOME` or an `XDG_*` variable itself.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The system data directories when `$XDG_DATA_DIRS` names none (the XDG base directory
/// specification's own default).
const DEFAULT_DATA_DIRS: [&str; 2] = ["/usr/local/share", "/usr/share"];

/// The folder old icon themes and the few programs that predate the data directories still use:
/// `/usr/share/pixmaps`, where an icon lookup ends.
const SYSTEM_PIXMAPS: &str = "/usr/share/pixmaps";

/// Where the person's files, settings, state, caches and session sockets live.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dirs {
    /// `$HOME`.
    pub home: PathBuf,
    /// `$XDG_CONFIG_HOME`, else `~/.config`.
    pub config: PathBuf,
    /// `$XDG_DATA_HOME`, else `~/.local/share`.
    pub data: PathBuf,
    /// `$XDG_DATA_DIRS`, in precedence order, else `/usr/local/share` and `/usr/share`. A
    /// relative entry names no fixed place and is left out.
    pub data_dirs: Vec<PathBuf>,
    /// `$XDG_STATE_HOME`, else `~/.local/state`.
    pub state: PathBuf,
    /// `$XDG_CACHE_HOME`, else `~/.cache`.
    pub cache: PathBuf,
    /// `$XDG_RUNTIME_DIR`: none outside a login session.
    pub runtime: Option<PathBuf>,
    /// The XDG Pictures folder (`user-dirs.dirs`), when one is named.
    pub pictures: Option<PathBuf>,
}

impl Dirs {
    /// This process's, as the `dirs` crate reads them. Without a home at all (no `$HOME` and
    /// no passwd entry) everything hangs off `/`, where writes fail and are logged.
    pub fn from_process() -> Dirs {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        Dirs {
            config: dirs::config_dir().unwrap_or_else(|| home.join(".config")),
            data: dirs::data_dir().unwrap_or_else(|| home.join(".local/share")),
            data_dirs: data_dirs_from(std::env::var_os("XDG_DATA_DIRS")),
            state: dirs::state_dir().unwrap_or_else(|| home.join(".local/state")),
            cache: dirs::cache_dir().unwrap_or_else(|| home.join(".cache")),
            runtime: dirs::runtime_dir(),
            pictures: dirs::picture_dir(),
            home,
        }
    }

    /// Every directory inside `root` (a test's scratch folder): `root/home`, `root/config`, ...,
    /// `root/run`; no Pictures folder.
    pub fn under(root: &Path) -> Dirs {
        Dirs {
            home: root.join("home"),
            config: root.join("config"),
            data: root.join("data"),
            data_dirs: vec![root.join("share")],
            state: root.join("state"),
            cache: root.join("cache"),
            runtime: Some(root.join("run")),
            pictures: None,
        }
    }

    /// Every folder an icon file may be named from: the icon themes and the pixmaps of the data
    /// directory and of each system data directory, `~/.icons`, and the system pixmaps. The
    /// lookup that finds an icon by name and the shell's host (which reads only inside these)
    /// agree through this one list.
    pub fn icon_dirs(&self) -> Vec<PathBuf> {
        let data = std::iter::once(&self.data).chain(&self.data_dirs);
        let mut dirs: Vec<PathBuf> = data
            .flat_map(|dir| [dir.join("icons"), dir.join("pixmaps")])
            .collect();
        dirs.push(self.home.join(".icons"));
        dirs.push(PathBuf::from(SYSTEM_PIXMAPS));
        dirs
    }

    /// `sill`'s own folder under the state directory.
    pub fn sill_state(&self) -> PathBuf {
        self.state.join("sill")
    }

    /// `sill`'s own folder under the cache directory.
    pub fn sill_cache(&self) -> PathBuf {
        self.cache.join("sill")
    }

    /// `sill`'s own folder under the config directory (the settings file, the dock pins).
    pub fn sill_config(&self) -> PathBuf {
        self.config.join("sill")
    }

    /// keycap's folder under the config directory: the person's `keycap.toml` is in it.
    pub fn keycap_config(&self) -> PathBuf {
        self.config.join("keycap")
    }
}

/// The directories `$XDG_DATA_DIRS` (`raw`) names: colon-separated, absolute ones only; the
/// specification's default when it is unset or names none.
fn data_dirs_from(raw: Option<OsString>) -> Vec<PathBuf> {
    let named: Vec<PathBuf> = raw
        .iter()
        .flat_map(std::env::split_paths)
        .filter(|dir| dir.is_absolute())
        .collect();
    match named.is_empty() {
        true => DEFAULT_DATA_DIRS.iter().map(PathBuf::from).collect(),
        false => named,
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::{Dirs, data_dirs_from};

    #[test]
    fn a_scratch_root_holds_every_directory() {
        let dirs = Dirs::under(Path::new("/scratch"));
        for dir in [
            &dirs.home,
            &dirs.config,
            &dirs.data,
            &dirs.state,
            &dirs.cache,
        ] {
            assert!(dir.starts_with("/scratch"), "{dir:?}");
        }
        assert_eq!(dirs.runtime.as_deref(), Some(Path::new("/scratch/run")));
        assert_eq!(dirs.pictures, None);
        assert_eq!(dirs.sill_state(), Path::new("/scratch/state/sill"));
        assert_eq!(dirs.keycap_config(), Path::new("/scratch/config/keycap"));
    }

    #[test]
    fn data_dirs_are_the_absolute_entries_or_the_default() {
        const CASES: &[(Option<&str>, &[&str])] = &[
            (None, &["/usr/local/share", "/usr/share"]),
            (Some(""), &["/usr/local/share", "/usr/share"]),
            (Some("a:b"), &["/usr/local/share", "/usr/share"]),
            (Some("/opt/x:/usr/share"), &["/opt/x", "/usr/share"]),
            (Some("/opt/x::rel:/y"), &["/opt/x", "/y"]),
        ];
        for (raw, want) in CASES {
            let got = data_dirs_from(raw.map(OsString::from));
            let want: Vec<PathBuf> = want.iter().map(PathBuf::from).collect();
            assert_eq!(got, want, "{raw:?}");
        }
    }

    #[test]
    fn icon_dirs_list_themes_and_pixmaps_of_every_data_dir_then_home_and_system() {
        let dirs = Dirs::under(Path::new("/s"));
        let want: Vec<PathBuf> = [
            "/s/data/icons",
            "/s/data/pixmaps",
            "/s/share/icons",
            "/s/share/pixmaps",
            "/s/home/.icons",
            "/usr/share/pixmaps",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        assert_eq!(dirs.icon_dirs(), want);
    }
}
