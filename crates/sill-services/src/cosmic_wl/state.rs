//! The thread's Wayland state, and the only file that names sctk or cctk delegate macros.

use cosmic_client_toolkit::cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::ZcosmicToplelevelManagementCapabilitiesV1 as ManagerAbility;
use cosmic_client_toolkit::sctk::output::OutputState;
use cosmic_client_toolkit::sctk::registry::{ProvidesRegistryState, RegistryState};
use cosmic_client_toolkit::sctk::seat::SeatState;
use cosmic_client_toolkit::sctk::{delegate_output, delegate_registry, delegate_seat, registry_handlers};
use cosmic_client_toolkit::toplevel_info::ToplevelInfoState;
use cosmic_client_toolkit::toplevel_management::ToplevelManagerState;
use cosmic_client_toolkit::wayland_client::QueueHandle;
use cosmic_client_toolkit::wayland_client::globals::GlobalList;
use cosmic_client_toolkit::wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;
use cosmic_client_toolkit::wayland_protocols::ext::workspace::v1::client::ext_workspace_group_handle_v1::ExtWorkspaceGroupHandleV1;
use cosmic_client_toolkit::wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::ExtWorkspaceHandleV1;
use cosmic_client_toolkit::workspace::WorkspaceState;
use cosmic_client_toolkit::{delegate_toplevel_info, delegate_toplevel_manager, delegate_workspace};

use crate::cosmic_wl::keys::Keyring;
use crate::cosmic_wl::publish::Publish;

/// Everything the thread's event queue dispatches into. (No `Debug`: cctk's
/// `ToplevelManagerState` has none.)
pub(crate) struct WlState {
    pub(crate) registry: RegistryState,
    pub(crate) seats: SeatState,
    /// Bound so toplevels and workspace groups can name their outputs (`DP-1`).
    pub(crate) outputs: OutputState,
    /// `None` on a compositor without the COSMIC toplevel protocols.
    pub(crate) toplevel_info: Option<ToplevelInfoState>,
    pub(crate) toplevel_manager: Option<ToplevelManagerState>,
    /// What the toplevel manager said it can do; empty until its `capabilities` event.
    pub(crate) manager_abilities: Vec<ManagerAbility>,
    pub(crate) workspaces: WorkspaceState,
    pub(crate) keys: Keys,
    pub(crate) publish: Publish,
}

/// The keys handed out so far, one ring per kind of handle.
#[derive(Debug, Default)]
pub(crate) struct Keys {
    pub(crate) windows: Keyring<ExtForeignToplevelHandleV1>,
    pub(crate) spaces: Keyring<ExtWorkspaceHandleV1>,
    pub(crate) groups: Keyring<ExtWorkspaceGroupHandleV1>,
}

/// Which COSMIC protocols the compositor offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Offer {
    /// Toplevels, workspaces, or both: worth running.
    Cosmic,
    /// Neither (KWin): nothing will ever be published.
    Nothing,
}

impl WlState {
    /// Bind every global the thread uses. Absent COSMIC globals are not an error here; see
    /// [`WlState::offer`].
    pub(crate) fn bind(
        globals: &GlobalList,
        qh: &QueueHandle<WlState>,
        publish: Publish,
    ) -> WlState {
        let registry = RegistryState::new(globals);
        WlState {
            seats: SeatState::new(globals, qh),
            outputs: OutputState::new(globals, qh),
            toplevel_info: ToplevelInfoState::try_new(&registry, qh),
            toplevel_manager: ToplevelManagerState::try_new(&registry, qh),
            manager_abilities: Vec::new(),
            workspaces: WorkspaceState::new(&registry, qh),
            registry,
            keys: Keys::default(),
            publish,
        }
    }

    pub(crate) fn offer(&self) -> Offer {
        let workspaces = self.workspaces.workspace_manager().get().is_ok();
        if self.toplevel_info.is_some() || workspaces {
            Offer::Cosmic
        } else {
            Offer::Nothing
        }
    }

    /// Give the watch channels back so the thread can park with them.
    pub(crate) fn into_publish(self) -> Publish {
        self.publish
    }
}

impl ProvidesRegistryState for WlState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    registry_handlers!(OutputState, SeatState);
}

delegate_registry!(WlState);
delegate_seat!(WlState);
delegate_output!(WlState);
delegate_toplevel_info!(WlState);
delegate_toplevel_manager!(WlState);
delegate_workspace!(WlState);
