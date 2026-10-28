//! Wire toplevels into sill's windows, most recently activated first.

use desktop_client::DesktopState;
use desktop_client::toplevel::{Toplevel as WireToplevel, ToplevelFlag, ToplevelFlags};
use sill_launcher::WindowKey;
use sill_model::ids::WorkspaceKey;
use sill_model::windows::Activity;
use sill_model::windows::toplevels::{
    Sizing, Toplevel, ToplevelEvent, Toplevels, WindowState, step,
};

use super::ids::{output_name, window_key, workspace_key};
use super::view::identifier;

/// `previous` brought up to `state`: windows that are gone close, every other window is
/// stepped in, so a window that has just become active moves to the front and one that has
/// just opened goes to the back. `previous` carries the recency the protocol does not.
pub fn project(previous: Toplevels, state: &DesktopState) -> Toplevels {
    let now: Vec<Toplevel> = state
        .toplevels
        .announced()
        .map(|toplevel| window(toplevel, state))
        .collect();
    let gone: Vec<WindowKey> = previous
        .windows
        .iter()
        .map(|w| w.key)
        .filter(|key| !now.iter().any(|w| w.key == *key))
        .collect();
    let closed = gone
        .into_iter()
        .fold(previous, |all, key| step(all, ToplevelEvent::Closed(key)));
    now.into_iter()
        .fold(closed, |all, w| step(all, ToplevelEvent::Changed(w)))
}

fn window(toplevel: &WireToplevel, state: &DesktopState) -> Toplevel {
    Toplevel {
        identifier: identifier(toplevel),
        key: window_key(toplevel.key),
        app_id: toplevel.app_id.clone(),
        title: toplevel.title.clone(),
        activity: activity(&toplevel.flags),
        window: window_state(&toplevel.flags),
        sizing: sizing(&toplevel.flags),
        workspaces: shown_on(toplevel, state),
        outputs: toplevel.outputs.iter().map(output_name).collect(),
    }
}

fn activity(flags: &ToplevelFlags) -> Activity {
    if flags.contains(ToplevelFlag::Activated) {
        Activity::Active
    } else {
        Activity::Inactive
    }
}

fn window_state(flags: &ToplevelFlags) -> WindowState {
    if flags.contains(ToplevelFlag::Minimized) {
        WindowState::Minimized
    } else {
        WindowState::Normal
    }
}

/// Full screen wins over maximised, as the shell reads it.
fn sizing(flags: &ToplevelFlags) -> Sizing {
    if flags.contains(ToplevelFlag::Fullscreen) {
        Sizing::Fullscreen
    } else if flags.contains(ToplevelFlag::Maximized) {
        Sizing::Maximized
    } else {
        Sizing::Floating
    }
}

/// The standard protocols do not say which workspace a window is on, so a window is on the
/// active workspaces of the groups that show its outputs (every active workspace when it names
/// no output, or the workspace is in no group).
fn shown_on(toplevel: &WireToplevel, state: &DesktopState) -> Vec<WorkspaceKey> {
    state
        .workspaces
        .active()
        .filter(|workspace| {
            let group = workspace
                .group
                .and_then(|key| state.workspaces.groups().find(|g| g.key == key));
            match (group, toplevel.outputs.is_empty()) {
                (Some(group), false) => group.outputs.iter().any(|o| toplevel.outputs.contains(o)),
                _ => true,
            }
        })
        .map(|workspace| workspace_key(workspace.key))
        .collect()
}
