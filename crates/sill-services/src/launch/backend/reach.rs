//! What the launch service reaches: the session bus and the user's systemd manager.

use std::path::PathBuf;

use crate::env::{Bus, Env};

/// What the launch service reaches: D-Bus activation and the file manager on `session`, the
/// user's systemd manager under `runtime` (`$XDG_RUNTIME_DIR`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Reach {
    pub(crate) session: Bus,
    pub(crate) runtime: Option<PathBuf>,
}

impl Reach {
    /// `env`'s session bus and runtime directory.
    pub(crate) fn of(env: &Env) -> Reach {
        Reach {
            session: env.buses.session.clone(),
            runtime: env.dirs.runtime.clone(),
        }
    }
}
