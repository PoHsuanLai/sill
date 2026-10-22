//! The shell's commands as the wire client's: a [`WlCommand`] first in the desktop's words
//! ([`DesktopCommand`]), then in the protocols'.

use casement_protocols::vocab::ShellRole;
use desktop_client::toplevel::ToplevelFlag;
use desktop_client::workspace::WorkspaceAbility;
use desktop_client::{DesktopCommand as Wire, DesktopState};
use porter_core::SpaceId;
use sill_model::desktop::{DesktopBackend, DesktopCommand, Supports, supports};
use sill_model::windows::WlCommand;

use super::ids::{space_id, toplevel_key, wire_workspace_key};
use super::view::identifier;

/// Why a command did not become wire requests.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// The standard protocols cannot do it.
    #[error("the standard protocols cannot carry this command")]
    Unsupported,
    /// No open window has this identifier.
    #[error("no window {0}")]
    UnknownWindow(String),
    /// No workspace has this Space id.
    #[error("no Space {0}")]
    UnknownSpace(SpaceId),
    /// No running app has this key.
    #[error("no app {0}")]
    UnknownApp(String),
    /// No surface is registered for this role, so there is nothing to name.
    #[error("no surface for the {0:?}")]
    NoSurface(ShellRole),
}

/// `command` as the desktop's command. `None` for the requests no arm takes (reordering,
/// pinning), and for a window or workspace `state` does not know.
pub fn desktop_of(command: &WlCommand, state: &DesktopState) -> Option<DesktopCommand> {
    let window = |key| state.toplevels.get(toplevel_key(key)).map(identifier);
    match command {
        WlCommand::Activate(key) => window(*key).map(DesktopCommand::Activate),
        WlCommand::Close(key) => window(*key).map(DesktopCommand::Close),
        WlCommand::Minimize(key) => window(*key).map(DesktopCommand::Minimise),
        WlCommand::Unminimize(key) => window(*key).map(DesktopCommand::Unminimise),
        WlCommand::ActivateWorkspace(key) => state
            .workspaces
            .get(wire_workspace_key(*key))
            .map(|w| DesktopCommand::SwitchSpace(space_id(w))),
        WlCommand::MoveWorkspace { .. }
        | WlCommand::PinWorkspace(_)
        | WlCommand::UnpinWorkspace(_)
        | WlCommand::MoveToplevelToWorkspace { .. } => None,
    }
}

/// The wire requests that carry `command`, in order. Activating a minimised window restores
/// it first; showing a Space is the workspace request and the commit that applies it.
pub fn wire_of(command: &DesktopCommand, state: &DesktopState) -> Result<Vec<Wire>, Refusal> {
    if supports(DesktopBackend::Wlr, command) == Supports::No {
        return Err(Refusal::Unsupported);
    }
    match command {
        DesktopCommand::Activate(id) => {
            let key = find_window(state, id)?;
            let minimised = state
                .toplevels
                .get(key)
                .is_some_and(|t| t.flags.contains(ToplevelFlag::Minimized));
            let restore = minimised.then_some(Wire::Unminimise(key));
            Ok(restore.into_iter().chain([Wire::Activate(key)]).collect())
        }
        DesktopCommand::Close(id) => Ok(vec![Wire::Close(find_window(state, id)?)]),
        DesktopCommand::Minimise(id) => Ok(vec![Wire::Minimise(find_window(state, id)?)]),
        DesktopCommand::Unminimise(id) => Ok(vec![Wire::Unminimise(find_window(state, id)?)]),
        DesktopCommand::SwitchSpace(space) => switch(state, space),
        // The Quire arm carries these: `quire::quire_wire_of`.
        DesktopCommand::Place(..)
        | DesktopCommand::ToSpace(..)
        | DesktopCommand::SetMinimiseTarget { .. }
        | DesktopCommand::ClearMinimiseTarget(_)
        | DesktopCommand::AssignSpaces(..)
        | DesktopCommand::ActivateApp(_)
        | DesktopCommand::HideApp(_)
        | DesktopCommand::UnhideApp(_)
        | DesktopCommand::HideOtherApps(_)
        | DesktopCommand::QuitApp(_)
        | DesktopCommand::ForceQuitApp(_)
        | DesktopCommand::ReopenApp(_)
        | DesktopCommand::CreateSpace(_)
        | DesktopCommand::RemoveSpace(_)
        | DesktopCommand::MoveSpace { .. }
        | DesktopCommand::Overview { .. }
        | DesktopCommand::ShowDesktop(_)
        | DesktopCommand::PickSwitcher(_)
        | DesktopCommand::CancelSwitcher
        | DesktopCommand::PickWindowMenu(..)
        | DesktopCommand::PickSplit(_)
        | DesktopCommand::CancelSplit
        | DesktopCommand::DivideSplit(..)
        | DesktopCommand::CaptureSpace { .. }
        | DesktopCommand::CaptureOutput { .. }
        | DesktopCommand::SetSpaceTargets { .. }
        | DesktopCommand::SetNewSpaceTarget { .. }
        | DesktopCommand::AnswerPermission { .. }
        | DesktopCommand::AnswerTrial { .. }
        | DesktopCommand::SetFullScreenBar(_)
        | DesktopCommand::SetMotion(_) => Err(Refusal::Unsupported),
    }
}

pub(super) fn find_window(
    state: &DesktopState,
    id: &str,
) -> Result<desktop_client::ToplevelKey, Refusal> {
    state
        .toplevels
        .announced()
        .find(|t| identifier(t) == id)
        .map(|t| t.key)
        .ok_or_else(|| Refusal::UnknownWindow(id.to_owned()))
}

/// Activate the workspace `space` names, when the compositor lets a client activate it.
fn switch(state: &DesktopState, space: &SpaceId) -> Result<Vec<Wire>, Refusal> {
    let workspace = state
        .workspaces
        .workspaces()
        .find(|w| &space_id(w) == space)
        .ok_or_else(|| Refusal::UnknownSpace(space.clone()))?;
    match workspace.abilities.contains(&WorkspaceAbility::Activate) {
        true => Ok(vec![
            Wire::ActivateWorkspace(workspace.key),
            Wire::CommitWorkspaces,
        ]),
        false => Err(Refusal::Unsupported),
    }
}
