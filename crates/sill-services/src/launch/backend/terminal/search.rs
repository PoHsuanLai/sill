//! Where the choice of terminal looks: the person's lists, the desktop entries and `PATH`, all
//! taken from `Env` so a test points them at a scratch folder.

use std::path::PathBuf;

use crate::env::Env;

/// The places the default terminal is looked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerminalSearch {
    /// The desktop's names from `XDG_CURRENT_DESKTOP`, lower case, most specific first.
    pub(crate) desktops: Vec<String>,
    /// The configuration folders, the person's first.
    pub(crate) config: Vec<PathBuf>,
    /// The data folders, the person's first; their `applications` hold the desktop entries.
    pub(crate) data: Vec<PathBuf>,
    /// The folders a program named without a folder is found in.
    pub(crate) programs: Vec<PathBuf>,
}

impl TerminalSearch {
    /// `env`'s folders and desktop.
    pub(crate) fn of(env: &Env) -> TerminalSearch {
        let first_then = |first: &PathBuf, rest: &[PathBuf]| {
            std::iter::once(first.clone())
                .chain(rest.iter().cloned())
                .collect()
        };
        TerminalSearch {
            desktops: env
                .current_desktop
                .split(':')
                .filter(|name| !name.is_empty())
                .map(str::to_lowercase)
                .collect(),
            config: first_then(&env.dirs.config, &env.dirs.config_dirs),
            data: first_then(&env.dirs.data, &env.dirs.data_dirs),
            programs: env.dirs.program_dirs.clone(),
        }
    }
}
