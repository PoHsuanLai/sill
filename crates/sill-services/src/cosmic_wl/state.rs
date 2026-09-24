//! The thread's Wayland state: the only file (with `handlers.rs`) that names sctk delegate
//! macros.

#![allow(unused_variables, dead_code)] // Freeze stubs

use cosmic_client_toolkit::sctk::registry::RegistryState;
use cosmic_client_toolkit::sctk::seat::SeatState;
use cosmic_client_toolkit::toplevel_info::ToplevelInfoState;
use cosmic_client_toolkit::toplevel_management::ToplevelManagerState;
use cosmic_client_toolkit::workspace::WorkspaceState;

/// Everything the thread's event queue dispatches into.
pub(crate) struct WlState {
    pub(crate) registry: RegistryState,
    pub(crate) seats: SeatState,
    /// `None` on a compositor without the COSMIC toplevel protocols.
    pub(crate) toplevel_info: Option<ToplevelInfoState>,
    pub(crate) toplevel_manager: Option<ToplevelManagerState>,
    pub(crate) workspaces: WorkspaceState,
}
