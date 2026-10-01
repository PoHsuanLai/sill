//! The person's directories, read from the process once ([`Dirs::from_process`]) or set by a
//! test ([`Dirs::under`]), so no service reads `HOME` or an `XDG_*` variable itself.

use std::path::{Path, PathBuf};

/// Where the person's files, settings, state, caches and session sockets live.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Dirs {
    /// `$HOME`.
    pub home: PathBuf,
    /// `$XDG_CONFIG_HOME`, else `~/.config`.
    pub config: PathBuf,
    /// `$XDG_DATA_HOME`, else `~/.local/share`.
    pub data: PathBuf,
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
            state: root.join("state"),
            cache: root.join("cache"),
            runtime: Some(root.join("run")),
            pictures: None,
        }
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Dirs;

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
}
