//! The look of a workspace and of the active one on an output (design/21 §5): the preset the
//! workspace's position picks and the card accent from `spaces.*`. Every surface
//! draws its `Ds { look }` from [`use_active_look`], so an output follows its own active
//! workspace (rule 6).
//!
//! quire has no `spaces.json` store yet (design/21 §10 names `ds-settings`' writer); until it
//! does [`use_space_look`] returns the design/21 §4 default for the workspace's index:
//! `PRESETS[index % 8]` and `spaces.default_card_accent`.

use ds::{CardAccent, SpaceLook, default_look};
use shell_host::OutputInfo;
use sill_model::cosmic_wl::{Activity, WorkspaceGroup, Workspaces};
use sill_model::ids::{OutputName, WorkspaceKey};
use sill_services::CosmicWlService;
use sill_settings::{DefaultCardAccent, SpacesSettings};

use crate::hooks::{use_service_part, use_settings};

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
    group_for(&workspaces.groups, output).and_then(|group| {
        group
            .workspaces
            .iter()
            .position(|w| w.activity == Activity::Active)
            .map(|index| (group.workspaces[index].key, index))
    })
}

/// The look of the active workspace on `output` (design/21 §5 rule 6); the first preset while
/// there is none (KWin).
pub fn active_look(
    workspaces: &Workspaces,
    output: Option<&OutputName>,
    spaces: &SpacesSettings,
) -> SpaceLook {
    let (_, index) = active_workspace(workspaces, output).unwrap_or((WorkspaceKey(0), 0));
    use_space_look(index, spaces)
}

/// The look of the active workspace on `output`, live.
pub fn use_active_look(output: &OutputInfo) -> SpaceLook {
    let workspaces = use_service_part::<CosmicWlService, _>(|wl| wl.workspaces.clone());
    let settings = use_settings();
    let on = output.name.clone().map(OutputName);
    active_look(&workspaces(), on.as_ref(), &settings().spaces)
}

/// The look of the workspace at position `index` in its group, with `spaces` defaults. With no
/// store every workspace gets the look of its index.
pub fn use_space_look(index: usize, spaces: &SpacesSettings) -> SpaceLook {
    default_look(index, card_accent(spaces.default_card_accent))
}

/// `spaces.default_card_accent` in quire's words.
fn card_accent(accent: DefaultCardAccent) -> CardAccent {
    match accent {
        DefaultCardAccent::Chosen => CardAccent::Chosen,
        DefaultCardAccent::SpaceHue => CardAccent::SpaceHue,
    }
}

#[cfg(test)]
mod tests {
    use super::use_space_look;
    use ds::{CardAccent, PRESETS, Theme};
    use sill_settings::{DefaultCardAccent, SpacesSettings};

    #[test]
    fn a_workspace_without_a_look_takes_its_index_preset() {
        let spaces = SpacesSettings {
            default_card_accent: DefaultCardAccent::SpaceHue,
            ..SpacesSettings::default()
        };
        // (index, preset): the preset repeats every eight workspaces.
        const CASES: &[(usize, usize)] = &[(0, 0), (1, 1), (2, 2), (9, 1)];
        for &(index, preset) in CASES {
            let look = use_space_look(index, &spaces);
            assert_eq!(look.dots, PRESETS[preset].dots.to_vec(), "index {index}");
            assert_eq!(look.theme, Theme::System, "index {index}");
            assert_eq!(look.card_accent, CardAccent::SpaceHue, "index {index}");
        }
    }
}
