//! The closed set of things the shell does when asked as `org.quire.Shell`.

use sill_launcher::AppId;

use crate::screenshots::ShotKind;
use crate::windows::Step;

/// One action of `org.quire.Shell`, its parameters parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellAction {
    /// `shell.launcher.open`: show the launcher with `query` typed (empty: a fresh one).
    LauncherOpen { query: String },
    /// `shell.dock.pin`: keep `app` (a desktop id) in the dock.
    DockPin { app: AppId },
    /// `shell.dock.unpin`: stop keeping `app`.
    DockUnpin { app: AppId },
    /// `shell.workspace.switch`: one workspace over.
    WorkspaceSwitch(Step),
    /// `shell.lock`: lock the screen now.
    Lock,
    /// `shell.screenshot`: take one with the person's screenshot tool.
    Screenshot(ShotKind),
}

/// Why the shell did not do an action it understood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellFault {
    /// This build or session cannot do it.
    Unsupported,
    /// It failed; the words say what, never the argument's text.
    Failed(String),
}
