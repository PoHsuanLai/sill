//! The compositor's toplevels and workspaces as the shell sees them: keys, words, capabilities, the
//! commands the compositor can be sent, and the pure steps that fold each protocol event in.

mod app;
mod caps;
mod command;
mod ids;
mod model;
mod subjects;
pub mod toplevels;
pub mod workspaces;

pub use app::{AppMatch, app_for, app_of};
pub use caps::{WorkspaceCap, WorkspaceCaps};
pub use command::{Relative, WlCommand};
pub use ids::{Activity, Pinning};
pub use model::WindowModel;
pub use subjects::Subjects;
pub use toplevels::{Sizing, Toplevel, ToplevelEvent, Toplevels, WindowState};
pub use workspaces::{Step, Workspace, WorkspaceEvent, WorkspaceGroup, Workspaces};
