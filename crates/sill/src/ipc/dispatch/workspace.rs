//! `sill workspace prev|next|overview`: palmrest's swipes. A step activates the neighbour of the
//! active workspace through the compositor service; at either end nothing happens. The overview
//! opens Mission Control on the desktop's own compositor, and is refused on any other.

use sill_ipc::act::WorkspaceRequest;
use sill_ipc::response::Response;
use sill_model::desktop::{DesktopCommand, open_mission_control};
use sill_model::windows::{Step, WlCommand, Workspaces};

use super::reach::{Carried, Reach};
use super::reply::{sent, unsupported};

/// Carry out `request` against the compositor service's current workspaces.
pub fn step(reach: &Reach, request: WorkspaceRequest) -> Response {
    let handle = &reach.services.windows;
    let plan = plan(&handle.state.borrow().workspaces, request);
    match plan {
        Plan::Send(command) => sent(handle.control.send(command), "windows"),
        Plan::Stay => Response::Done,
        Plan::Ask(command) => asked(reach.desktop.send(command)),
    }
}

/// The reply once the compositor was asked.
fn asked(carried: Carried) -> Response {
    match carried {
        Carried::Yes => Response::Done,
        Carried::No => unsupported("this session's compositor has no Mission Control"),
    }
}

/// What a request comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Plan {
    Send(WlCommand),
    /// Already at the end: a swipe past it is not an error.
    Stay,
    /// Ask the desktop's own compositor.
    Ask(DesktopCommand),
}

fn plan(workspaces: &Workspaces, request: WorkspaceRequest) -> Plan {
    let step = match request {
        WorkspaceRequest::Prev => Step::Prev,
        WorkspaceRequest::Next => Step::Next,
        WorkspaceRequest::Overview => return Plan::Ask(open_mission_control()),
    };
    workspaces.neighbour(step).map_or(Plan::Stay, |key| {
        Plan::Send(WlCommand::ActivateWorkspace(key))
    })
}

#[cfg(test)]
mod tests {
    use sill_ipc::act::WorkspaceRequest;
    use sill_ipc::response::{FailureKind, Response};
    use sill_model::desktop::open_mission_control;
    use sill_model::ids::{GroupKey, OutputName, WorkspaceKey};
    use sill_model::units::{Urgency, Visibility};
    use sill_model::windows::{
        Activity, Pinning, WlCommand, Workspace, WorkspaceCaps, WorkspaceGroup, Workspaces,
    };

    use super::{Carried, Plan, asked, plan};

    fn row(active: u64) -> Workspaces {
        let ws = |k: u64| Workspace {
            key: WorkspaceKey(k),
            name: k.to_string(),
            stable_id: String::new(),
            activity: if k == active {
                Activity::Active
            } else {
                Activity::Inactive
            },
            urgency: Urgency::Calm,
            visibility: Visibility::Shown,
            coordinates: Vec::new(),
            capabilities: WorkspaceCaps::default(),
            pinning: Pinning::Unpinned,
        };
        Workspaces {
            groups: vec![WorkspaceGroup {
                key: GroupKey(1),
                outputs: vec![OutputName("HDMI-A-1".to_owned())],
                workspaces: (1..=3).map(ws).collect(),
            }],
        }
    }

    #[test]
    fn requests_become_plans() {
        let go = |k| Plan::Send(WlCommand::ActivateWorkspace(WorkspaceKey(k)));
        let cases = [
            (row(2), WorkspaceRequest::Next, go(3)),
            (row(2), WorkspaceRequest::Prev, go(1)),
            (row(3), WorkspaceRequest::Next, Plan::Stay),
            (row(1), WorkspaceRequest::Prev, Plan::Stay),
            (Workspaces::default(), WorkspaceRequest::Next, Plan::Stay),
            (
                row(2),
                WorkspaceRequest::Overview,
                Plan::Ask(open_mission_control()),
            ),
        ];
        for (all, request, want) in cases {
            assert_eq!(plan(&all, request), want, "{request:?}");
        }
    }

    #[test]
    fn the_overview_is_refused_without_the_desktops_compositor() {
        assert_eq!(asked(Carried::Yes), Response::Done);
        let Response::Failed(failure) = asked(Carried::No) else {
            panic!("a refusal"); // test-only
        };
        assert_eq!(failure.kind, FailureKind::Unsupported);
    }
}
