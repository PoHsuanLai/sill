//! The real host: the windows service for the windows, `systemctl --user` for the scopes.

use std::path::PathBuf;
use std::time::Duration;

use sill_launcher::WindowKey;
use sill_model::windows::{WindowModel, WlCommand};
use tokio::sync::watch;

use super::{AppsHost, OpenWindow};
use crate::control::Control;
use crate::launch::{Manager, user_manager};

/// The scopes sill starts every app in.
const APP_SCOPES: &str = "app-sill-*.scope";

/// The windows service and the user's systemd manager, as the apps' host.
pub(crate) struct LiveApps {
    pub(crate) windows: watch::Receiver<WindowModel>,
    pub(crate) wl: Control<WlCommand>,
    /// `$XDG_RUNTIME_DIR`; without a user manager under it no scope is touched.
    pub(crate) runtime: Option<PathBuf>,
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
        if user_manager(self.runtime.as_deref()) == Manager::Missing {
            return;
        }
        let _ = tokio::process::Command::new("systemctl")
            .args(["--user", "kill", "--signal=SIGTERM", APP_SCOPES])
            .status()
            .await;
    }
}
