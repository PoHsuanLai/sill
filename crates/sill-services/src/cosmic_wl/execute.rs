//! Carrying out a [`WlCommand`] on the thread's connection. A command for an unknown key, or
//! one the compositor did not say it supports, is dropped: the published state is the answer.

use cosmic_client_toolkit::cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::{
    State as WindowFlag, ZcosmicToplevelHandleV1,
};
use cosmic_client_toolkit::cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::{
    ZcosmicToplelevelManagementCapabilitiesV1 as ManagerAbility, ZcosmicToplevelManagerV1,
};
use cosmic_client_toolkit::cosmic_protocols::workspace::v2::client::zcosmic_workspace_handle_v2::WorkspaceCapabilities as CosmicAbilities;
use cosmic_client_toolkit::toplevel_info::ToplevelInfo;
use cosmic_client_toolkit::wayland_client::Proxy;
use cosmic_client_toolkit::wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::WorkspaceCapabilities as Abilities;
use cosmic_client_toolkit::workspace::Workspace as WorkspaceInfo;
use sill_launcher::WindowKey;

use crate::cosmic_wl::command::{Relative, WlCommand};
use crate::cosmic_wl::ids::WorkspaceKey;
use crate::cosmic_wl::state::WlState;
use crate::cosmic_wl::workspaces::WorkspaceEvent;

/// The coordinate a workspace move is measured along. COSMIC's groups are one row, so their
/// coordinates have one component.
const ROW_AXIS: u32 = 0;

/// `zcosmic_workspace_handle_v2` gained `move_before`/`move_after` in version 2.
const MOVE_SINCE: u32 = 2;

pub(super) fn execute(state: &WlState, command: WlCommand) {
    match command {
        WlCommand::Activate(key) => activate(state, key),
        WlCommand::Close(key) => {
            if let Some((manager, window, _)) = managed(state, key, ManagerAbility::Close) {
                manager.close(&window);
            }
        }
        WlCommand::Minimize(key) => {
            if let Some((manager, window, _)) = managed(state, key, ManagerAbility::Minimize) {
                manager.set_minimized(&window);
            }
        }
        WlCommand::Unminimize(key) => {
            if let Some((manager, window, _)) = managed(state, key, ManagerAbility::Minimize) {
                manager.unset_minimized(&window);
            }
        }
        WlCommand::ActivateWorkspace(key) => activate_workspace(state, key),
        WlCommand::MoveWorkspace {
            workspace,
            relative,
            other,
        } => move_workspace(state, workspace, relative, other),
    }
}

/// Activate on the first seat; a minimized window is restored first, so the dock's click on a
/// minimized window's tile brings it back (design/10 §10.3).
fn activate(state: &WlState, key: WindowKey) {
    let Some((manager, window, info)) = managed(state, key, ManagerAbility::Activate) else {
        return;
    };
    let Some(seat) = state.seats.seats().next() else {
        return;
    };
    if info.state.contains(&WindowFlag::Minimized)
        && state.manager_abilities.contains(&ManagerAbility::Minimize)
    {
        manager.unset_minimized(&window);
    }
    manager.activate(&window, &seat);
}

/// The manager, the COSMIC handle and the description of `key`, if the manager offers
/// `ability`.
fn managed(
    state: &WlState,
    key: WindowKey,
    ability: ManagerAbility,
) -> Option<(
    &ZcosmicToplevelManagerV1,
    ZcosmicToplevelHandleV1,
    &ToplevelInfo,
)> {
    if !state.manager_abilities.contains(&ability) {
        return None;
    }
    let manager = &state.toplevel_manager.as_ref()?.manager;
    let handle = state.keys.windows.handle(key.0)?;
    let info = state.toplevel_info.as_ref()?.info(handle)?;
    let window = info.cosmic_toplevel.clone()?;
    Some((manager, window, info))
}

fn activate_workspace(state: &WlState, key: WorkspaceKey) {
    let Some(workspace) = workspace(state, key) else {
        return;
    };
    if !workspace.capabilities.contains(Abilities::Activate) {
        return;
    }
    workspace.handle.activate();
    commit(state);
    state.publish.workspace(WorkspaceEvent::Activated(key));
}

fn move_workspace(state: &WlState, key: WorkspaceKey, relative: Relative, other: WorkspaceKey) {
    let (Some(moved), Some(anchor)) = (workspace(state, key), workspace(state, other)) else {
        return;
    };
    let Some(cosmic) = moved.cosmic_handle.as_ref() else {
        return;
    };
    if cosmic.version() < MOVE_SINCE || !moved.cosmic_capabilities.contains(CosmicAbilities::Move) {
        return;
    }
    match relative {
        Relative::Before => cosmic.move_before(&anchor.handle, ROW_AXIS),
        Relative::After => cosmic.move_after(&anchor.handle, ROW_AXIS),
    }
    commit(state);
}

fn workspace(state: &WlState, key: WorkspaceKey) -> Option<&WorkspaceInfo> {
    let handle = state.keys.spaces.handle(key.0)?;
    state.workspaces.workspace_info(handle)
}

/// ext-workspace requests take effect only on the manager's `commit`.
fn commit(state: &WlState) {
    if let Ok(manager) = state.workspaces.workspace_manager().get() {
        manager.commit();
    }
}
