//! cctk and sctk handler impls for [`WlState`]: each turns a protocol callback into a
//! [`ToplevelEvent`] or a [`WorkspaceEvent`] and publishes the reduced state. The delegate
//! macros that route events here are in `state.rs`.

use cosmic_client_toolkit::cosmic_protocols::toplevel_management::v1::client::zcosmic_toplevel_manager_v1::ZcosmicToplelevelManagementCapabilitiesV1 as ManagerAbility;
use cosmic_client_toolkit::sctk::output::{OutputHandler, OutputState};
use cosmic_client_toolkit::sctk::seat::{Capability, SeatHandler, SeatState};
use cosmic_client_toolkit::toplevel_info::{ToplevelInfoHandler, ToplevelInfoState};
use cosmic_client_toolkit::toplevel_management::{ToplevelManagerHandler, ToplevelManagerState};
use cosmic_client_toolkit::wayland_client::{Connection, QueueHandle, WEnum};
use cosmic_client_toolkit::wayland_client::protocol::wl_output::WlOutput;
use cosmic_client_toolkit::wayland_client::protocol::wl_seat::WlSeat;
use cosmic_client_toolkit::wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;
use cosmic_client_toolkit::workspace::{WorkspaceHandler, WorkspaceState};
use sill_launcher::WindowKey;
use sill_model::cosmic_wl::toplevels::{Toplevel, ToplevelEvent};
use sill_model::cosmic_wl::workspaces::{WorkspaceEvent, WorkspaceGroup, Workspaces};
use sill_model::ids::{GroupKey, OutputName, WorkspaceKey};

use super::describe;
use super::state::{Keys, WlState};

impl WlState {
    /// The toplevel behind `handle` as it stands after its latest `done`.
    fn describe_toplevel(&mut self, handle: &ExtForeignToplevelHandleV1) -> Option<Toplevel> {
        let info = self.toplevel_info.as_ref()?.info(handle)?;
        let key = WindowKey(self.keys.windows.key(handle));
        let mut workspaces: Vec<WorkspaceKey> = info
            .workspace
            .iter()
            .map(|w| WorkspaceKey(self.keys.spaces.key(w)))
            .collect();
        workspaces.sort();
        let outputs = output_names(&self.outputs, info.output.iter());
        Some(describe::toplevel(key, info, workspaces, outputs))
    }

    fn publish_toplevel(&mut self, handle: &ExtForeignToplevelHandleV1, opened: Opening) {
        if let Some(window) = self.describe_toplevel(handle) {
            self.publish.toplevel(match opened {
                Opening::New => ToplevelEvent::Opened(window),
                Opening::Known => ToplevelEvent::Changed(window),
            });
        }
    }

    fn publish_workspaces(&mut self) {
        let view = describe_workspaces(&self.workspaces, &self.outputs, &mut self.keys);
        self.publish.workspace(WorkspaceEvent::Replaced(view));
    }
}

/// Whether a toplevel's description is its first.
enum Opening {
    New,
    Known,
}

/// Every group and its workspaces in the compositor's group order, each group's workspaces by
/// coordinates. Keys of workspaces and groups that no longer exist are dropped.
fn describe_workspaces(
    state: &WorkspaceState,
    outputs: &OutputState,
    keys: &mut Keys,
) -> Workspaces {
    keys.spaces
        .retain(|h| state.workspaces().any(|w| &w.handle == h));
    keys.groups
        .retain(|h| state.workspace_groups().any(|g| &g.handle == h));
    let groups = state
        .workspace_groups()
        .map(|group| {
            let mut members: Vec<_> = state
                .workspaces()
                .filter(|w| group.workspaces.contains(&w.handle))
                .collect();
            members.sort_by(|a, b| a.coordinates.cmp(&b.coordinates));
            WorkspaceGroup {
                key: GroupKey(keys.groups.key(&group.handle)),
                outputs: output_names(outputs, group.outputs.iter()),
                workspaces: members
                    .into_iter()
                    .map(|w| describe::workspace(WorkspaceKey(keys.spaces.key(&w.handle)), w))
                    .collect(),
            }
        })
        .collect();
    Workspaces { groups }
}

/// The connector names of `outputs` that have one yet, sorted.
fn output_names<'a>(
    state: &OutputState,
    outputs: impl Iterator<Item = &'a WlOutput>,
) -> Vec<OutputName> {
    let mut names: Vec<OutputName> = outputs
        .filter_map(|o| state.info(o)?.name)
        .map(OutputName)
        .collect();
    names.sort();
    names
}

impl ToplevelInfoHandler for WlState {
    fn toplevel_info_state(&mut self) -> &mut ToplevelInfoState {
        // cctk dispatches toplevel-info events only to objects bound by `try_new`, which
        // returned `Some`: an event here proves the state exists.
        self.toplevel_info
            .as_mut()
            .expect("toplevel-info event without a bound toplevel-info state") // invariant above
    }

    fn new_toplevel(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        handle: &ExtForeignToplevelHandleV1,
    ) {
        self.publish_toplevel(handle, Opening::New);
    }

    fn update_toplevel(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        handle: &ExtForeignToplevelHandleV1,
    ) {
        self.publish_toplevel(handle, Opening::Known);
    }

    fn toplevel_closed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        handle: &ExtForeignToplevelHandleV1,
    ) {
        if let Some(key) = self.keys.windows.existing(handle) {
            self.keys.windows.forget(handle);
            self.publish.toplevel(ToplevelEvent::Closed(WindowKey(key)));
        }
    }
}

impl ToplevelManagerHandler for WlState {
    fn toplevel_manager_state(&mut self) -> &mut ToplevelManagerState {
        // As above: manager events reach only a manager `try_new` bound.
        self.toplevel_manager
            .as_mut()
            .expect("toplevel-manager event without a bound manager") // invariant above
    }

    fn capabilities(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        capabilities: Vec<WEnum<ManagerAbility>>,
    ) {
        self.manager_abilities = capabilities
            .into_iter()
            .filter_map(|c| c.into_result().ok())
            .collect();
    }
}

impl WorkspaceHandler for WlState {
    fn workspace_state(&mut self) -> &mut WorkspaceState {
        &mut self.workspaces
    }

    fn done(&mut self) {
        self.publish_workspaces();
    }
}

impl SeatHandler for WlState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}

    fn new_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat, _: Capability) {}

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: WlSeat,
        _: Capability,
    ) {
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlSeat) {}
}

/// An output's name can arrive after the workspaces that sit on it, so the view is described
/// again whenever an output changes.
impl OutputHandler for WlState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {
        self.publish_workspaces();
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {
        self.publish_workspaces();
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: WlOutput) {
        self.publish_workspaces();
    }
}
