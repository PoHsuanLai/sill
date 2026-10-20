//! Workspaces per group, and their one reducer.

use serde::{Deserialize, Serialize};

use super::{Activity, Pinning, WorkspaceCaps};
use crate::ids::{GroupKey, OutputName, WorkspaceKey};
use crate::units::Urgency;
use crate::units::Visibility;

/// Every workspace group and its workspaces, in the compositor's order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Workspaces {
    pub groups: Vec<WorkspaceGroup>,
}

/// A group of workspaces: one per output, or one shared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceGroup {
    pub key: GroupKey,
    pub outputs: Vec<OutputName>,
    /// In position order (the bar's indicator order; 21-SPACES §4 "index").
    pub workspaces: Vec<Workspace>,
}

/// One workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub key: WorkspaceKey,
    pub name: String,
    /// The protocol's stable id when it has one (ext-workspace `id`): the `by_id` key of
    /// `spaces.json` (21-SPACES §10). Empty when the compositor gives none.
    pub stable_id: String,
    pub activity: Activity,
    pub urgency: Urgency,
    pub visibility: Visibility,
    /// The protocol's coordinates in its group (one component, the row position). The
    /// group's `workspaces` are already in this order; a surface needs the numbers only to
    /// place a workspace in a grid. Empty when the compositor gives none.
    #[serde(default)]
    pub coordinates: Vec<u32>,
    /// The requests the compositor accepts for this workspace; a command outside it is dropped
    /// by the thread.
    #[serde(default)]
    pub capabilities: WorkspaceCaps,
    /// The pinned state; `Unpinned` where the compositor has no such state.
    #[serde(default)]
    pub pinning: Pinning,
}

/// One change to publish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum WorkspaceEvent {
    /// The thread's whole view after a protocol `done`.
    Replaced(Workspaces),
    /// We asked for this workspace: mark it active in its group ahead of the compositor's
    /// `done`, so the indicator moves on the press's frame. The next `Replaced` is the truth.
    Activated(WorkspaceKey),
}

/// `workspaces` after `event`. Activating an unknown key changes nothing.
pub fn step(workspaces: Workspaces, event: WorkspaceEvent) -> Workspaces {
    match event {
        WorkspaceEvent::Replaced(all) => all,
        WorkspaceEvent::Activated(key) => Workspaces {
            groups: workspaces
                .groups
                .into_iter()
                .map(|group| activate_in(group, key))
                .collect(),
        },
    }
}

/// `group` with `key` active and its siblings inactive, if `key` is one of its workspaces.
fn activate_in(group: WorkspaceGroup, key: WorkspaceKey) -> WorkspaceGroup {
    if !group.workspaces.iter().any(|w| w.key == key) {
        return group;
    }
    WorkspaceGroup {
        workspaces: group
            .workspaces
            .into_iter()
            .map(|w| Workspace {
                activity: if w.key == key {
                    Activity::Active
                } else {
                    Activity::Inactive
                },
                ..w
            })
            .collect(),
        ..group
    }
}

/// Which way `sill workspace` steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Prev,
    Next,
}

impl Workspaces {
    /// The workspace one `step` away from the active one, in the first group that has an active
    /// workspace (the main display's, in the compositor's order). There is no wrap-around: at
    /// the first or last workspace this is `None`, as keyboard shortcuts stop there.
    pub fn neighbour(&self, step: Step) -> Option<WorkspaceKey> {
        let group = self.main_group()?;
        let at = group.active_index()?;
        let to = match step {
            Step::Prev => at.checked_sub(1)?,
            Step::Next => at + 1,
        };
        group.workspaces.get(to).map(|w| w.key)
    }

    /// The group an index or a step counts in: the first with an active workspace (the main
    /// display's, in the compositor's order), else the first.
    pub fn main_group(&self) -> Option<&WorkspaceGroup> {
        self.groups
            .iter()
            .find(|g| g.active_index().is_some())
            .or_else(|| self.groups.first())
    }

    /// The group showing on `output`, if any.
    pub fn group_on(&self, output: &OutputName) -> Option<&WorkspaceGroup> {
        self.groups.iter().find(|g| g.outputs.contains(output))
    }

    /// The active workspace of the group on `output`.
    pub fn active_on(&self, output: &OutputName) -> Option<&Workspace> {
        self.group_on(output)?.active()
    }
}

impl WorkspaceGroup {
    /// The position of the active workspace in this group.
    pub fn active_index(&self) -> Option<usize> {
        self.workspaces
            .iter()
            .position(|w| w.activity == Activity::Active)
    }

    /// The active workspace of this group.
    pub fn active(&self) -> Option<&Workspace> {
        self.workspaces.get(self.active_index()?)
    }
}

#[cfg(test)]
mod tests {
    use super::{Step, Workspace, WorkspaceEvent, WorkspaceGroup, Workspaces, step};
    use crate::ids::{GroupKey, OutputName, WorkspaceKey};
    use crate::units::Urgency;
    use crate::units::Visibility;
    use crate::windows::{Activity, Pinning};
    use crate::windows::{WorkspaceCap, WorkspaceCaps};

    fn ws(key: u64, activity: Activity) -> Workspace {
        Workspace {
            key: WorkspaceKey(key),
            name: key.to_string(),
            stable_id: String::new(),
            activity,
            urgency: Urgency::Calm,
            visibility: Visibility::Shown,
            coordinates: vec![u32::try_from(key).unwrap_or(u32::MAX)],
            capabilities: [WorkspaceCap::Activate, WorkspaceCap::Pin]
                .into_iter()
                .collect::<WorkspaceCaps>(),
            pinning: Pinning::Unpinned,
        }
    }

    fn two_outputs(active_left: u64, active_right: u64) -> Workspaces {
        let group = |key, output: &str, keys: [u64; 2], active| WorkspaceGroup {
            key: GroupKey(key),
            outputs: vec![OutputName(output.to_owned())],
            workspaces: keys
                .iter()
                .map(|&k| {
                    ws(
                        k,
                        if k == active {
                            Activity::Active
                        } else {
                            Activity::Inactive
                        },
                    )
                })
                .collect(),
        };
        Workspaces {
            groups: vec![
                group(1, "DP-1", [1, 2], active_left),
                group(2, "DP-2", [3, 4], active_right),
            ],
        }
    }

    #[test]
    fn events_reduce_to_the_expected_workspaces() {
        let cases: Vec<(&str, Workspaces, WorkspaceEvent, Workspaces)> = vec![
            (
                "activation moves within its own group only",
                two_outputs(1, 3),
                WorkspaceEvent::Activated(WorkspaceKey(2)),
                two_outputs(2, 3),
            ),
            (
                "activating the active one changes nothing",
                two_outputs(1, 3),
                WorkspaceEvent::Activated(WorkspaceKey(1)),
                two_outputs(1, 3),
            ),
            (
                "activating an unknown key changes nothing",
                two_outputs(1, 3),
                WorkspaceEvent::Activated(WorkspaceKey(99)),
                two_outputs(1, 3),
            ),
            (
                "a replacement is the truth",
                two_outputs(2, 3),
                WorkspaceEvent::Replaced(two_outputs(1, 4)),
                two_outputs(1, 4),
            ),
        ];
        for (name, before, event, want) in cases {
            assert_eq!(step(before, event), want, "{name}");
        }
    }

    #[test]
    fn activation_keeps_coordinates_capabilities_and_pinning() {
        let before = two_outputs(1, 3);
        let after = step(before.clone(), WorkspaceEvent::Activated(WorkspaceKey(2)));
        let facts = |all: &Workspaces| -> Vec<_> {
            all.groups
                .iter()
                .flat_map(|g| g.workspaces.iter())
                .map(|w| {
                    (
                        w.key,
                        w.coordinates.clone(),
                        w.capabilities.clone(),
                        w.pinning,
                    )
                })
                .collect()
        };
        assert_eq!(facts(&after), facts(&before));
        assert_ne!(
            after, before,
            "the activation itself must have changed something"
        );
    }

    #[test]
    fn a_workspace_without_the_amended_fields_still_decodes() {
        // The shape published before these fields were added.
        let old = r#"{"key":4,"name":"4","stable_id":"","activity":"active","urgency":"calm","visibility":"shown"}"#;
        let decoded: Workspace = serde_json::from_str(old).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(decoded.coordinates, Vec::<u32>::new());
        assert_eq!(decoded.capabilities, WorkspaceCaps::default());
        assert_eq!(decoded.pinning, Pinning::Unpinned);
    }

    #[test]
    fn the_active_workspace_is_found_per_output() {
        let all = two_outputs(2, 3);
        let key_on = |o: &str| all.active_on(&OutputName(o.to_owned())).map(|w| w.key.0);
        assert_eq!(key_on("DP-1"), Some(2));
        assert_eq!(key_on("DP-2"), Some(3));
        assert_eq!(key_on("HDMI-1"), None);
    }

    #[test]
    fn stepping_stops_at_the_ends_and_uses_the_first_group() {
        let key = |all: &Workspaces, step| all.neighbour(step).map(|k| k.0);
        let cases = [
            (two_outputs(1, 3), Step::Next, Some(2)),
            (two_outputs(1, 3), Step::Prev, None),
            (two_outputs(2, 3), Step::Prev, Some(1)),
            (two_outputs(2, 3), Step::Next, None),
            (Workspaces::default(), Step::Next, None),
        ];
        for (all, step, want) in cases {
            assert_eq!(key(&all, step), want, "{step:?}");
        }
    }

    #[test]
    fn the_main_group_is_the_first_with_an_active_workspace_else_the_first() {
        let mut idle = two_outputs(1, 3);
        for w in &mut idle.groups[0].workspaces {
            w.activity = Activity::Inactive;
        }
        let main = |all: &Workspaces| all.main_group().map(|g| g.key.0);
        assert_eq!(main(&two_outputs(1, 3)), Some(1));
        assert_eq!(
            main(&idle),
            Some(2),
            "the first group with an active workspace"
        );
        idle.groups[1].workspaces[0].activity = Activity::Inactive;
        idle.groups[1].workspaces[1].activity = Activity::Inactive;
        assert_eq!(main(&idle), Some(1), "no active workspace: the first group");
        assert_eq!(main(&Workspaces::default()), None);
    }
}
