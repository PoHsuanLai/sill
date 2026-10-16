//! The standard-protocol arm of the compositor service: the desktop client's state (ext
//! toplevel list, wlr toplevel management, ext workspace) projected into the windows and
//! workspaces the shell draws, and the shell's commands as wire requests.
//!
//! The wire client is `desktop-client`; this module holds sill's part: recency (the protocols
//! give none), apps, Spaces and the commands' mapping. The thread starts in `thread`, which
//! picks the arm from the compositor's globals ([`backend_of`]).

mod choose;
mod command;
mod commands;
mod ids;
mod offer;
mod outputs;
mod publish;
pub mod quire;
mod run;
mod service;
mod subjects;
mod thread;
mod toplevels;
mod view;
mod workspaces;

pub use choose::backend_of;
pub use command::{Refusal, desktop_of, wire_of};
pub use ids::space_id;
pub use offer::{offer_of_protocols, offer_of_registry};
pub use outputs::{ScaleRefusal, config_for_scale, heads_of};
pub use service::WindowsService;
pub use subjects::project as project_subjects;
pub use toplevels::project as project_toplevels;
pub use view::desktop_view;
pub use workspaces::project as project_workspaces;
