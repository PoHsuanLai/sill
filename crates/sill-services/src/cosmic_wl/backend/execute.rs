//! Carrying out a [`WlCommand`] on the thread's connection. A command for an unknown key, or
//! one the compositor did not say it supports, is dropped: the published state is the answer.

use cosmic_client_toolkit::cosmic_protocols::toplevel_info::v1::client::zcosmic_toplevel_handle_v1::{State as WindowFlag, ZcosmicToplevelHandleV1};
use cosmic_client_toolkit::cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::{ZcosmicToplelevelManagementCapabilitiesV1 as ManagerAbility, ZcosmicToplevelManagerV1};
use cosmic_client_toolkit::cosmic_protocols::workspace::v2::client::zcosmic_workspace_handle_v2::WorkspaceCapabilities as CosmicAbilities;
use cosmic_client_toolkit::toplevel_info::ToplevelInfo;
use cosmic_client_toolkit::wayland_client::Proxy;
use cosmic_client_toolkit::wayland_client::protocol::wl_output::WlOutput;
use cosmic_client_toolkit::wayland_protocols::ext::workspace::v1::client::ext_workspace_handle_v1::{ExtWorkspaceHandleV1, WorkspaceCapabilities as Abilities};
use cosmic_client_toolkit::workspace::Workspace as WorkspaceInfo;
use sill_launcher::WindowKey;
use sill_model::cosmic_wl::{Relative, WlCommand, Pinning};
use sill_model::cosmic_wl::workspaces::WorkspaceEvent;
use sill_model::ids::WorkspaceKey;

use super::state::WlState;

/// The coordinate a workspace move is measured along. COSMIC's groups are one row, so their
/// coordinates have one component.
const ROW_AXIS: u32 = 0;

/// `zcosmic_workspace_handle_v2` gained `move_before`/`move_after` and `pin`/`unpin` in
/// version 2.
const COSMIC_V2_SINCE: u32 = 2;

/// `zcosmic_toplevel_manager_v1` gained `move_to_ext_workspace` in version 4.
const MOVE_TO_EXT_SINCE: u32 = 4;

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
        WlCommand::PinWorkspace(key) => pin_workspace(state, key, Pinning::Pinned),
        WlCommand::UnpinWorkspace(key) => pin_workspace(state, key, Pinning::Unpinned),
        WlCommand::MoveToplevelToWorkspace {
            toplevel,
            workspace,
        } => move_toplevel(state, toplevel, workspace),
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
    managed_any(state, key)
}

/// The manager, the COSMIC handle and the description of `key`, with the capability already
/// checked by the caller.
fn managed_any(
    state: &WlState,
    key: WindowKey,
) -> Option<(
    &ZcosmicToplevelManagerV1,
    ZcosmicToplevelHandleV1,
    &ToplevelInfo,
)> {
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
    if !cosmic_offers(
        cosmic.version(),
        moved.cosmic_capabilities,
        CosmicAbilities::Move,
    ) {
        return;
    }
    match relative {
        Relative::Before => cosmic.move_before(&anchor.handle, ROW_AXIS),
        Relative::After => cosmic.move_after(&anchor.handle, ROW_AXIS),
    }
    commit(state);
}

fn pin_workspace(state: &WlState, key: WorkspaceKey, pinning: Pinning) {
    let Some(workspace) = workspace(state, key) else {
        return;
    };
    let Some(cosmic) = workspace.cosmic_handle.as_ref() else {
        return;
    };
    if !cosmic_offers(
        cosmic.version(),
        workspace.cosmic_capabilities,
        CosmicAbilities::Pin,
    ) {
        return;
    }
    match pinning {
        Pinning::Pinned => cosmic.pin(),
        Pinning::Unpinned => cosmic.unpin(),
    }
    commit(state);
}

/// Move `toplevel` onto `key`, on the output of the workspace's group (the protocol needs one);
/// a group with no output yet falls back to an output the window is on.
fn move_toplevel(state: &WlState, toplevel: WindowKey, key: WorkspaceKey) {
    let Some(manager) = state.toplevel_manager.as_ref().map(|m| &m.manager) else {
        return;
    };
    if !offers_move_to_ext(manager.version(), &state.manager_abilities) {
        return;
    }
    let Some((manager, window, info)) = managed_any(state, toplevel) else {
        return;
    };
    let Some(target) = workspace(state, key) else {
        return;
    };
    let output = group_output(state, &target.handle).or_else(|| info.output.iter().next().cloned());
    if let Some(output) = output {
        manager.move_to_ext_workspace(&window, &target.handle, &output);
    }
}

/// The first output of the group that holds `workspace`.
fn group_output(state: &WlState, workspace: &ExtWorkspaceHandleV1) -> Option<WlOutput> {
    state
        .workspaces
        .workspace_groups()
        .find(|group| group.workspaces.contains(workspace))?
        .outputs
        .first()
        .cloned()
}

/// Whether a toplevel manager at `version` that advertised `offered` takes
/// `move_to_ext_workspace`. cosmic-comp 1.8 binds at version 4 and carries out
/// `move_to_ext_workspace`, but advertises only the older `move_to_workspace` capability, so at
/// version 4 either capability counts.
fn offers_move_to_ext(version: u32, offered: &[ManagerAbility]) -> bool {
    version >= MOVE_TO_EXT_SINCE
        && (offered.contains(&ManagerAbility::MoveToExtWorkspace)
            || offered.contains(&ManagerAbility::MoveToWorkspace))
}

/// Whether a `zcosmic_workspace_handle_v2` at `version` that advertised `offered` accepts the
/// `wanted` requests (every COSMIC workspace request this thread sends arrived in version 2).
/// Flags are compared whole: `pin` is the bit value 3.
fn cosmic_offers(version: u32, offered: CosmicAbilities, wanted: CosmicAbilities) -> bool {
    version >= COSMIC_V2_SINCE && offered.contains(wanted)
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

#[cfg(test)]
mod tests {
    use super::{CosmicAbilities, ManagerAbility, cosmic_offers, offers_move_to_ext};

    #[test]
    fn move_to_ext_workspace_is_gated_on_version_4_and_either_capability() {
        use ManagerAbility::{Activate, MoveToExtWorkspace, MoveToWorkspace};
        let cases: &[(&str, u32, &[ManagerAbility], bool)] = &[
            ("v4 with the ext capability", 4, &[MoveToExtWorkspace], true),
            (
                "cosmic-comp 1.8: v4, old capability only",
                4,
                &[Activate, MoveToWorkspace],
                true,
            ),
            ("v3 never", 3, &[MoveToExtWorkspace, MoveToWorkspace], false),
            ("v4 without either", 4, &[Activate], false),
        ];
        for (name, version, offered, want) in cases {
            assert_eq!(offers_move_to_ext(*version, offered), *want, "{name}");
        }
    }

    #[test]
    fn cosmic_requests_are_gated_on_version_and_capability() {
        let cases = [
            (
                "v2 offering pin",
                2,
                CosmicAbilities::Pin,
                CosmicAbilities::Pin,
                true,
            ),
            (
                "v1 never",
                1,
                CosmicAbilities::Pin,
                CosmicAbilities::Pin,
                false,
            ),
            (
                "rename alone is not pin",
                2,
                CosmicAbilities::Rename,
                CosmicAbilities::Pin,
                false,
            ),
            (
                "move offered",
                2,
                CosmicAbilities::Move,
                CosmicAbilities::Move,
                true,
            ),
            (
                "pin is not move",
                2,
                CosmicAbilities::Pin,
                CosmicAbilities::Move,
                false,
            ),
            (
                "nothing offered",
                2,
                CosmicAbilities::empty(),
                CosmicAbilities::Move,
                false,
            ),
        ];
        for (name, version, offered, wanted, want) in cases {
            assert_eq!(cosmic_offers(version, offered, wanted), want, "{name}");
        }
    }
}
