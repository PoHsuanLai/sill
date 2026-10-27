//! The look of a workspace and of the active one on an output (design/21 §5): the preset the
//! workspace's position picks and the grain a constant. Every surface
//! draws its `Ds { look }` from [`use_active_look`], so an output follows its own active
//! workspace (rule 6).
//!
//! quire has no `spaces.json` store yet (design/21 §10 names `ds-settings`' writer); until it
//! does [`use_space_look`] returns the design/21 §4 default for the workspace's index:
//! `PRESETS[index % 8]`, the preset's own grain (35 and 55 for presets 1 and 2) or
//! no grain. The card always wears the appearance accent.

use ds::prelude::*;
use ds::style::space::presets::default_look;
use shell_host::OutputInfo;
use sill_model::ids::{OutputName, WorkspaceKey};
use sill_model::windows::{WorkspaceGroup, Workspaces};
use sill_services::WindowsService;

use crate::hooks::use_service_part;

/// The group this output shows: the one naming it, or the first when the output has no name.
pub fn group_for(groups: &[WorkspaceGroup], output: Option<&OutputName>) -> Option<WorkspaceGroup> {
    match output {
        Some(name) => groups.iter().find(|g| g.outputs.contains(name)),
        None => groups.first(),
    }
    .cloned()
}

/// The active workspace of `output`'s group: its key and its position in the group.
pub fn active_workspace(
    workspaces: &Workspaces,
    output: Option<&OutputName>,
) -> Option<(WorkspaceKey, usize)> {
    let group = group_for(&workspaces.groups, output)?;
    let index = group.active_index()?;
    group.active().map(|w| (w.key, index))
}

/// The look of the active workspace on `output` (design/21 §5 rule 6); the first preset while
/// there is none (KWin).
pub fn active_look(workspaces: &Workspaces, output: Option<&OutputName>) -> SpaceLook {
    let (_, index) = active_workspace(workspaces, output).unwrap_or((WorkspaceKey(0), 0));
    use_space_look(index)
}

/// The look of the active workspace on `output`, live.
pub fn use_active_look(output: &OutputInfo) -> SpaceLook {
    let workspaces = use_service_part::<WindowsService, _>(|wl| wl.workspaces.clone());
    let on = output.name.clone().map(OutputName);
    active_look(&workspaces(), on.as_ref())
}

/// The grain of a Space that has no look of its own.
const DEFAULT_GRAIN: Grain = Grain(0);

/// The look of the workspace at position `index` in its group. With no store every workspace
/// gets the look of its index.
pub fn use_space_look(index: usize) -> SpaceLook {
    default_look(index, DEFAULT_GRAIN)
}

#[cfg(test)]
mod tests {
    use super::use_space_look;
    use ds::prelude::*;
    use ds::style::space::presets::PRESETS;

    #[test]
    fn a_workspace_without_a_look_takes_its_index_preset() {
        // (index, preset)
        const CASES: &[(usize, usize)] = &[(0, 0), (1, 1), (2, 2), (9, 1)];
        for &(index, preset) in CASES {
            let look = use_space_look(index);
            assert_eq!(look.dots, PRESETS[preset].dots.to_vec(), "index {index}");
            assert_eq!(look.grain, Grain(0), "index {index}");
            assert_eq!(look.theme, Theme::System, "index {index}");
        }
    }
}
