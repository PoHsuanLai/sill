//! What the windows service publishes: the compositor's workspaces and windows.

use super::toplevels::Toplevels;
use super::workspaces::Workspaces;

/// The compositor's workspaces and windows: one Wayland connection's state, published together
/// (a reader that wants one of them projects it).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WindowModel {
    pub workspaces: Workspaces,
    pub toplevels: Toplevels,
}
