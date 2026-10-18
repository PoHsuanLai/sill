//! What the launch service reaches: the session bus and the user's systemd manager.

use std::path::PathBuf;
use std::sync::Arc;

use super::TerminalSearch;
use crate::env::{Bus, Children, Env};

/// What the launch service reaches: D-Bus activation and the file manager on `session`, the
/// user's systemd manager under `runtime` (`$XDG_RUNTIME_DIR`), and the opener's children through
/// `children`; the terminal a `Terminal=true` app is wrapped in is looked for in `terminal`.
#[derive(Debug, Clone)]
pub(crate) struct Reach {
    pub(crate) session: Bus,
    pub(crate) runtime: Option<PathBuf>,
    pub(crate) children: Arc<dyn Children>,
    pub(crate) terminal: TerminalSearch,
}

impl Reach {
    /// `env`'s session bus, runtime directory and children.
    pub(crate) fn of(env: &Env) -> Reach {
        Reach {
            session: env.buses.session.clone(),
            runtime: env.dirs.runtime.clone(),
            children: env.children.clone(),
            terminal: TerminalSearch::of(env),
        }
    }
}
