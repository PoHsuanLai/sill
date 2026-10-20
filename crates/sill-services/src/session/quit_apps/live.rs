//! The real host: the windows service for the windows, `systemctl --user` (through
//! `Env::children`) for the scopes.

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use sill_launcher::WindowKey;
use sill_model::windows::{WindowModel, WlCommand};
use tokio::sync::watch;

use super::{AppsHost, OpenWindow};
use crate::control::Control;
use crate::env::{Children, ProcessGroup, Start};
use crate::launch::{Manager, user_manager};

/// The scopes sill starts every app in.
const APP_SCOPES: &str = "app-sill-*.scope";

/// How long the scopes get to take the signal before the session ends.
const SCOPE_GRACE: Duration = Duration::from_secs(1);

/// Where the apps' scopes are stopped: the user manager under `runtime`, by a child of `children`.
#[derive(Debug, Clone)]
pub(crate) struct Scopes {
    /// `$XDG_RUNTIME_DIR`; without a user manager under it no scope is touched.
    pub(crate) runtime: Option<PathBuf>,
    pub(crate) children: Arc<dyn Children>,
}

/// The windows service and the user's systemd manager, as the apps' host.
pub(crate) struct LiveApps {
    pub(crate) windows: watch::Receiver<WindowModel>,
    pub(crate) wl: Control<WlCommand>,
    pub(crate) scopes: Scopes,
}

impl AppsHost for LiveApps {
    fn open_windows(&self) -> Vec<OpenWindow> {
        self.windows
            .borrow()
            .toplevels
            .windows
            .iter()
            .map(|window| OpenWindow {
                key: window.key,
                app_id: window.app_id.clone(),
            })
            .collect()
    }

    fn close(&self, key: WindowKey) {
        // A stopped windows service has no windows left to close.
        let _ = self.wl.send(WlCommand::Close(key));
    }

    async fn wait(&self, span: Duration) {
        tokio::time::sleep(span).await;
    }

    async fn stop_scopes(&self) {
        if user_manager(self.scopes.runtime.as_deref()) == Manager::Missing {
            return;
        }
        let start = Start {
            argv: [
                "systemctl",
                "--user",
                "kill",
                "--signal=SIGTERM",
                APP_SCOPES,
            ]
            .map(OsString::from)
            .to_vec(),
            token: None,
            group: ProcessGroup::Shared,
        };
        // Without systemctl the session's end stops the scopes anyway.
        if self.scopes.children.detach(&start).is_ok() {
            tokio::time::sleep(SCOPE_GRACE).await;
        }
    }
}
