//! The reduction: the compositor's workspaces, each with the look the shell gives it.

use super::model::PublishedWorkspace;
use crate::cosmic_wl::{Activity, Workspaces};

/// Every workspace of `workspaces`, in order, with `look_of(index)` for the workspace at position
/// `index` in its group.
pub fn published(
    workspaces: &Workspaces,
    look_of: impl Fn(usize) -> String,
) -> Vec<PublishedWorkspace> {
    workspaces
        .groups
        .iter()
        .flat_map(|group| {
            let output = group
                .outputs
                .first()
                .map(|name| name.0.clone())
                .unwrap_or_default();
            let look_of = &look_of;
            group
                .workspaces
                .iter()
                .enumerate()
                .map(move |(index, workspace)| PublishedWorkspace {
                    id: match workspace.stable_id.is_empty() {
                        true => workspace.key.0.to_string(),
                        false => workspace.stable_id.clone(),
                    },
                    name: workspace.name.clone(),
                    output: output.clone(),
                    look: look_of(index),
                    active: workspace.activity == Activity::Active,
                })
        })
        .collect()
}
