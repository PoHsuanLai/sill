//! COSMIC's toplevels and workspaces as the shell sees them: keys, words, capabilities, the
//! commands the compositor can be sent, and the pure steps that fold each protocol event in.

mod caps;
mod command;
mod ids;
mod model;
pub mod toplevels;
pub mod workspaces;

pub use caps::{WorkspaceCap, WorkspaceCaps};
pub use command::{Relative, WlCommand};
pub use ids::{Activity, Pinning, Urgency};
pub use model::CosmicWl;
pub use toplevels::{Sizing, Toplevel, ToplevelEvent, Toplevels, WindowState};
pub use workspaces::{Workspace, WorkspaceEvent, WorkspaceGroup, Workspaces};
