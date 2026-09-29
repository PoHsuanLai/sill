//! Workspaces per group, and their one reducer.

use serde::{Deserialize, Serialize};

use super::{Activity, Pinning, Urgency, WorkspaceCaps};
use crate::ids::{GroupKey, OutputName, WorkspaceKey};
use crate::units::Visibility;

/// Every workspace group and its workspaces, in the compositor's order.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Workspaces {
    pub groups: Vec<WorkspaceGroup>,
}

/// A group of workspaces: one per output on COSMIC's default, or one shared.
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
    /// The protocol's coordinates in its group (COSMIC: one component, the row position). The
    /// group's `workspaces` are already in this order; a surface needs the numbers only to
    /// place a workspace in a grid. Empty when the compositor gives none.
    #[serde(default)]
    pub coordinates: Vec<u32>,
    /// The requests the compositor accepts for this workspace; a command outside it is dropped
    /// by the thread.
    #[serde(default)]
    pub capabilities: WorkspaceCaps,
    /// COSMIC's pinned state; `Unpinned` where the compositor has no such state.
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

impl Workspaces {
    /// The group showing on `output`, if any.
    pub fn group_on(&self, output: &OutputName) -> Option<&WorkspaceGroup> {
        self.groups.iter().find(|g| g.outputs.contains(output))
    }

    /// The active workspace of the group on `output`.
    pub fn active_on(&self, output: &OutputName) -> Option<&Workspace> {
        self.group_on(output)?
            .workspaces
            .iter()
            .find(|w| w.activity == Activity::Active)
    }
}

#[cfg(test)]
mod tests {
    use super::{Workspace, WorkspaceEvent, WorkspaceGroup, Workspaces, step};
    use crate::cosmic_wl::{Activity, Pinning, Urgency};
    use crate::cosmic_wl::{WorkspaceCap, WorkspaceCaps};
    use crate::ids::{GroupKey, OutputName, WorkspaceKey};
    use crate::units::Visibility;

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
}
