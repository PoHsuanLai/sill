//! COSMIC's toplevel and workspace state, on a thread with its own Wayland connection and cctk
//! states (plan "Design: `<shell>` repo": "`cosmic_wl` thread with its own wayland Connection +
//! cctk states"; FINDINGS "Findings: COSMIC", cctk API).
//!
//! The sctk/cctk delegate macros live in `state.rs` and `handlers.rs` only (plan shell risk
//! "sctk 0.20/0.21 -> macros confined to state.rs + handlers/"). On a compositor without the
//! COSMIC protocols (KWin) both receivers stay empty and commands are dropped: "the dock
//! degrades quietly on KWin" (design/10 §10.7).

#![allow(unused_variables, dead_code)] // Freeze stubs

mod command;
mod handlers;
mod ids;
mod state;
mod toplevels;
mod workspaces;

pub use command::{Relative, WlCommand};
pub use ids::{Activity, GroupKey, OutputName, Urgency, WorkspaceKey};
pub use toplevels::{Sizing, Toplevel, ToplevelEvent, Toplevels, WindowState, reduce_toplevels};
pub use workspaces::{Workspace, WorkspaceEvent, WorkspaceGroup, Workspaces, reduce_workspaces};

use tokio::sync::watch;

use crate::control::Control;

/// Start the `cosmic_wl` thread: connect to `$WAYLAND_DISPLAY`, bind the cctk globals, and
/// publish toplevels and workspaces after every `done`.
pub fn spawn() -> (
    watch::Receiver<Workspaces>,
    watch::Receiver<Toplevels>,
    Control<WlCommand>,
) {
    todo!(
        "M1 svc-cosmic: own Connection + calloop, ToplevelInfoState, ToplevelManagerState, WorkspaceState"
    )
}
