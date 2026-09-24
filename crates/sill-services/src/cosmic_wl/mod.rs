//! COSMIC's toplevel and workspace state, on a thread with its own Wayland connection and cctk
//! states (plan "Design: `<shell>` repo": "`cosmic_wl` thread with its own wayland Connection +
//! cctk states"; FINDINGS "Findings: COSMIC", cctk API).
//!
//! The sctk/cctk delegate macros live in `state.rs` only, the handler impls in `handlers.rs`
//! (plan shell risk "sctk 0.20/0.21 -> macros confined to state.rs + handlers/"). On a
//! compositor without the COSMIC protocols (KWin) both receivers stay empty, the thread parks
//! on the command channel and commands are dropped: "the dock degrades quietly on KWin"
//! (design/10 §10.7).

mod caps;
mod command;
mod describe;
mod execute;
mod handlers;
mod ids;
mod keys;
mod publish;
mod state;
mod thread;
mod toplevels;
mod workspaces;

#[cfg(test)]
mod nested;

pub use caps::{WorkspaceCap, WorkspaceCaps};
pub use command::{Relative, WlCommand};
pub use ids::{Activity, GroupKey, OutputName, Pinning, Urgency, WorkspaceKey};
pub use toplevels::{Sizing, Toplevel, ToplevelEvent, Toplevels, WindowState, reduce_toplevels};
pub use workspaces::{Workspace, WorkspaceEvent, WorkspaceGroup, Workspaces, reduce_workspaces};

use tokio::sync::watch;

use crate::control::Control;
use crate::cosmic_wl::publish::Publish;
use crate::cosmic_wl::thread::Endpoint;

/// Start the `cosmic_wl` thread: connect to `$WAYLAND_DISPLAY`, bind the cctk globals, and
/// publish toplevels and workspaces after every `done`.
pub fn spawn() -> (
    watch::Receiver<Workspaces>,
    watch::Receiver<Toplevels>,
    Control<WlCommand>,
) {
    spawn_on(Endpoint::Environment)
}

/// [`spawn`] against a chosen compositor.
fn spawn_on(
    endpoint: Endpoint,
) -> (
    watch::Receiver<Workspaces>,
    watch::Receiver<Toplevels>,
    Control<WlCommand>,
) {
    let (publish, workspaces, toplevels) = Publish::channels();
    let (control, commands) = Control::channel();
    let started = std::thread::Builder::new()
        .name("cosmic_wl".to_owned())
        .spawn(move || thread::run(endpoint, publish, commands));
    if let Err(error) = started {
        eprintln!("sill: cosmic_wl: no thread: {error}; toplevels and workspaces stay empty");
    }
    (workspaces, toplevels, control)
}
