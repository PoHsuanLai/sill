//! The workspace indicator: one pill per workspace of this output's group, active sealed, drag to reorder (`move_before`/`move_after`), right-click for Edit Space.
//!
//! The group is a ds `WorkspacePills` (the macOS polish pass: one segmented group on the frame),
//! each workspace a `WorkspacePill`, current for the active one. A primary press activates the
//! workspace; a press on one pill released over another reorders (`reorder`), which each pill's
//! own wrapper listens for. Right-click Edit Space waits for the Space editor popup (FINDINGS
//! "M1 bar-ui").

use dioxus::prelude::*;
use ds::{Here, PointerButton, Press, WorkspacePill, WorkspacePills};
use shell_host::OutputInfo;
use sill_services::{Activity, OutputName, WlCommand, Workspace, WorkspaceGroup};

use super::reorder::{Dropped, Pressed, reorder};
use super::title::pointer_at;
use crate::hooks::{use_services, use_settings, use_workspaces};

/// The group this output shows: the one naming it, or the first when the output has no name.
pub(crate) fn group_for(
    groups: &[WorkspaceGroup],
    output: Option<&OutputName>,
) -> Option<WorkspaceGroup> {
    match output {
        Some(name) => groups.iter().find(|g| g.outputs.contains(name)),
        None => groups.first(),
    }
    .cloned()
}

/// A pill's label: the workspace's name, or its position.
fn label(workspace: &Workspace, index: usize) -> String {
    if workspace.name.is_empty() {
        (index + 1).to_string()
    } else {
        workspace.name.clone()
    }
}

/// The workspace indicator: one pill per workspace of this output's group, active sealed, drag to reorder (`move_before`/`move_after`), right-click for Edit Space.
#[component]
pub fn WorkspaceIndicator(output: OutputInfo) -> Element {
    let workspaces = use_workspaces();
    let settings = use_settings();
    let wl = use_services().wl;
    let mut pressed = use_signal(|| None::<Pressed>);
    let threshold = settings().dock.hold_to_menu_move_px.0;
    let on = output.name.clone().map(OutputName);
    let Some(group) = group_for(&workspaces().groups, on.as_ref()) else {
        return rsx! {};
    };
    rsx! {
        div { class: "sill-bar-spaces",
            WorkspacePills { label: "Workspaces",
            for (index , workspace) in group.workspaces.into_iter().enumerate() {
                div {
                    key: "{workspace.key.0}",
                    class: "sill-bar-space",
                    onpointerdown: move |event| {
                        let x = pointer_at(&event).x;
                        pressed.set(Some(Pressed { key: workspace.key, index, x }));
                    },
                    onpointerup: {
                        let wl = wl.clone();
                        move |event| {
                            let x = pointer_at(&event).x;
                            let dropped = Dropped { key: workspace.key, index, x };
                            let command = pressed.take().and_then(|p| reorder(p, dropped, threshold));
                            if let Some(command) = command {
                                let _ = wl.send(command);
                            }
                        }
                    },
                    WorkspacePill {
                        label: label(&workspace, index),
                        current: if workspace.activity == Activity::Active { Here::Current } else { Here::Elsewhere },
                        onclick: {
                            let wl = wl.clone();
                            move |press: Press| {
                                if press.button == PointerButton::Primary {
                                    let _ = wl.send(WlCommand::ActivateWorkspace(workspace.key));
                                }
                            }
                        },
                    }
                }
            }
            }
        }
    }
}
