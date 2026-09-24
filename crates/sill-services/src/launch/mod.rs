//! Starting apps (plan "Launch": "activation token, DBusActivatable via
//! org.freedesktop.Application.Activate, Terminal=true wrapper, `systemd-run --user --scope
//! --collect --unit=app-sill-<id>-<rand>.scope` (fallback process group)").
//!
//! The activation token comes from shell-host (`SurfaceHandle::activation_token`) in the
//! daemon's `sill::launch`, which then sends a [`LaunchCommand`] here.

#![allow(unused_variables, dead_code)] // Freeze stubs

mod model;
mod plan;

pub use model::{LaunchOutcome, LaunchedScope, Launches, ScopeUnit, reduce_launches};
pub use plan::{launch_argv, scope_unit};

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sill_launcher::{AppId, DesktopAction};
use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::apps::AppIndex;
use crate::control::Control;

/// One launch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub app: AppId,
    pub action: DesktopAction,
    /// Dropped files (`%f`/`%F`).
    pub files: Vec<PathBuf>,
    /// Dropped URIs (`%u`/`%U`).
    pub uris: Vec<String>,
    /// The xdg-activation token for `XDG_ACTIVATION_TOKEN` / `platform-data`; `None` when the
    /// compositor refused one.
    pub token: Option<String>,
}

/// What the launch service can be told.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum LaunchCommand {
    Launch(LaunchRequest),
    /// `org.freedesktop.FileManager1.ShowItems([desktop file uri])` (dock Cmd-click).
    ShowInFiles(AppId),
    /// SIGKILL the app's `app-sill-*` scopes (dock Force Quit).
    ForceQuit(AppId),
}

/// Start the launch service; `apps` resolves ids to entries.
pub fn spawn(
    rt: &Handle,
    apps: watch::Receiver<AppIndex>,
) -> (watch::Receiver<Launches>, Control<LaunchCommand>) {
    todo!("M2 launch: D-Bus Activate or systemd-run scope, track scopes -> reduce_launches")
}
