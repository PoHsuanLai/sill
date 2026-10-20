//! The person's directories, read from the process once ([`Dirs::from_process`]) or set by a
//! test ([`Dirs::under`]), so no service reads `HOME` or an `XDG_*` variable itself.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use ds_helpers::XdgEnv;

/// The system configuration directories when `$XDG_CONFIG_DIRS` names none.
const DEFAULT_CONFIG_DIRS: [&str; 1] = ["/etc/xdg"];

/// The folder old icon themes and the few programs that predate the data directories still use:
/// `/usr/share/pixmaps`, where an icon lookup ends.
const SYSTEM_PIXMAPS: &str = "/usr/share/pixmaps";

/// The folder app stores install programs under, outside the data directories.
const OPT: &str = "/opt";

/// The person's own `/opt` under the home folder, where a program installed without root puts
/// itself (Zen's own installer does).
const HOME_OPT: &str = ".local/opt";

/// The folder snap keeps each installed snap in, `<name>/current` being its active revision.
const SNAP: &str = "/snap";

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
    /// `$XDG_CONFIG_DIRS`, in precedence order, else `/etc/xdg`. A relative entry is left out.
    pub config_dirs: Vec<PathBuf>,
    /// `$PATH`, in order: where a program named without a folder is found. A relative entry is
    /// left out.
    pub program_dirs: Vec<PathBuf>,
    /// `$XDG_STATE_HOME`, else `~/.local/state`.
    pub state: PathBuf,
    /// `$XDG_CACHE_HOME`, else `~/.cache`.
    pub cache: PathBuf,
    /// `$XDG_RUNTIME_DIR`: none outside a login session.
    pub runtime: Option<PathBuf>,
    /// The XDG Pictures folder (`user-dirs.dirs`), when one is named.
    pub pictures: Option<PathBuf>,
    /// The app stores' own folders, where a desktop entry may name its icon by absolute path:
    /// `/opt`, `~/.local/opt` and each snap's `current` folder, listed once when the process started (a snap
    /// installed later is seen at the next start).
    pub app_stores: Vec<PathBuf>,
    /// The folders of desktop entries, most important first: the person's, each system data
    /// directory's, then the app stores' exports (Flatpak's installs, snap's; Ubuntu's Firefox is
    /// a snap). [`ds_helpers::XdgEnv::applications_dirs`] decides it, for every part of the
    /// desktop alike.
    pub applications: Vec<PathBuf>,
}

impl Dirs {
    /// This process's, as the `dirs` crate reads them. Without a home at all (no `$HOME` and
    /// no passwd entry) everything hangs off `/`, where writes fail and are logged.
    pub fn from_process() -> Dirs {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"));
        let xdg = XdgEnv::from_process();
        Dirs {
            config: dirs::config_dir().unwrap_or_else(|| home.join(".config")),
            data: xdg.data_home().unwrap_or_else(|| home.join(".local/share")),
            data_dirs: xdg.data_dirs(),
            applications: xdg.applications_dirs(),
            config_dirs: config_dirs_from(std::env::var_os("XDG_CONFIG_DIRS")),
            program_dirs: absolute_dirs(std::env::var_os("PATH")),
            state: dirs::state_dir().unwrap_or_else(|| home.join(".local/state")),
            cache: dirs::cache_dir().unwrap_or_else(|| home.join(".cache")),
            runtime: dirs::runtime_dir(),
            pictures: dirs::picture_dir(),
            app_stores: app_stores_in(&[PathBuf::from(OPT), home.join(HOME_OPT)], Path::new(SNAP)),
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
            config_dirs: vec![root.join("xdg")],
            program_dirs: vec![root.join("bin")],
            state: root.join("state"),
            cache: root.join("cache"),
            runtime: Some(root.join("run")),
            pictures: None,
            app_stores: vec![root.join("opt"), root.join("home").join(HOME_OPT)],
            applications: vec![
                root.join("data/applications"),
                root.join("share/applications"),
            ],
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

    /// Every folder an absolute `Icon=` of a desktop entry may name a file in: the icon folders
    /// ([`Dirs::icon_dirs`]) and the app stores' ([`Dirs::app_stores`]). The scan of the apps
    /// and the host's file roots take it from here, so what the scan keeps is what the host serves.
    pub fn icon_file_roots(&self) -> Vec<PathBuf> {
        let mut roots = self.icon_dirs();
        roots.extend(self.app_stores.clone());
        roots
    }

    /// Every folder a notification's picture may be read from: the icon folders
    /// ([`Dirs::icon_dirs`]) and `notifications` in sill's cache folder.
    pub fn notification_image_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = self.icon_dirs();
        dirs.push(self.sill_cache().join("notifications"));
        dirs
    }

    /// The freedesktop thumbnail cache, `thumbnails` in the cache directory: the pictures other
    /// programs (a file manager, a thumbnailer) made of files, by the size's folder.
    pub fn thumbnails(&self) -> PathBuf {
        self.cache.join("thumbnails")
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

/// `opts` and the `current` folder of each snap under `snap`, in name order; a snap with no
/// `current` folder is left out.
fn app_stores_in(opts: &[PathBuf], snap: &Path) -> Vec<PathBuf> {
    let mut currents: Vec<PathBuf> = std::fs::read_dir(snap)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("current"))
        .filter(|current| current.is_dir())
        .collect();
    currents.sort();
    opts.iter().cloned().chain(currents).collect()
}

/// The absolute directories a colon-separated list (`raw`) names, in order.
fn absolute_dirs(raw: Option<OsString>) -> Vec<PathBuf> {
    raw.iter()
        .flat_map(std::env::split_paths)
        .filter(|dir| dir.is_absolute())
        .collect()
}

/// `named`, or `default` when `named` is empty.
fn or_default(named: Vec<PathBuf>, default: &[&str]) -> Vec<PathBuf> {
    match named.is_empty() {
        true => default.iter().map(PathBuf::from).collect(),
        false => named,
    }
}

/// The directories `$XDG_CONFIG_DIRS` (`raw`) names: colon-separated, absolute ones only; the
/// specification's default when it is unset or names none.
fn config_dirs_from(raw: Option<OsString>) -> Vec<PathBuf> {
    or_default(absolute_dirs(raw), &DEFAULT_CONFIG_DIRS)
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::{Dirs, app_stores_in};

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
    fn config_dirs_are_the_absolute_entries_or_etc_xdg() {
        const CASES: &[(Option<&str>, &[&str])] = &[
            (None, &["/etc/xdg"]),
            (Some("rel"), &["/etc/xdg"]),
            (Some("/a:rel:/b"), &["/a", "/b"]),
        ];
        for (raw, want) in CASES {
            let got = super::config_dirs_from(raw.map(OsString::from));
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

    #[test]
    fn an_absolute_icon_may_lie_in_the_icon_folders_then_the_app_stores() {
        let dirs = Dirs::under(Path::new("/s"));
        let mut want = dirs.icon_dirs();
        want.push(PathBuf::from("/s/opt"));
        want.push(PathBuf::from("/s/home/.local/opt"));
        assert_eq!(dirs.icon_file_roots(), want);
    }

    #[test]
    fn app_stores_are_both_opts_and_each_snaps_current_folder() {
        let scratch = sill_testkit::TempDir::new("app-stores");
        let snap = scratch.path().join("snap");
        for current in [
            "firefox/current",
            "vlc/current",
            "bare/123",
            "core22/current",
        ] {
            std::fs::create_dir_all(snap.join(current)).unwrap_or_else(|e| panic!("{e}"));
        }
        std::fs::write(snap.join("README"), b"").unwrap_or_else(|e| panic!("{e}"));
        let opt = [
            scratch.path().join("opt"),
            scratch.path().join("home/.local/opt"),
        ];
        let want: Vec<PathBuf> = [
            opt[0].clone(),
            opt[1].clone(),
            snap.join("core22/current"),
            snap.join("firefox/current"),
            snap.join("vlc/current"),
        ]
        .into();
        assert_eq!(app_stores_in(&opt, &snap), want);
        assert_eq!(
            app_stores_in(&opt, &scratch.path().join("no-snap")),
            opt.to_vec(),
            "a machine without snap has only the two opts"
        );
    }

    #[test]
    fn the_thumbnail_cache_is_thumbnails_in_the_cache_directory() {
        assert_eq!(
            Dirs::under(Path::new("/s")).thumbnails(),
            Path::new("/s/cache/thumbnails")
        );
    }
}
