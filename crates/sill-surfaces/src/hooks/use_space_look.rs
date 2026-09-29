//! `use_space_look`: the SpaceLook of a workspace (design/21 §5 trigger: "`use_workspaces` ->
//! each surface's `Ds { look }`").
//!
//! quire has no `spaces.json` store yet (design/21 §10 names `ds-settings`' writer); until it
//! does this returns the design/21 §4 default for the workspace's index:
//! `PRESETS[index % 8]`, the preset's own grain (35 and 55 for presets 1 and 2) or
//! `spaces.default_grain`, and `spaces.default_card_accent`.

use ds::{CardAccent, Grain, SpaceLook, default_look};
use sill_model::ids::WorkspaceKey;
use sill_settings::{DefaultCardAccent, SpacesSettings};

/// The look of `workspace` at position `index` in its group, with `spaces` defaults. The key
/// is the future `by_id` lookup (`spaces.lookup_order`); with no store every workspace falls
/// through to its index.
pub fn use_space_look(workspace: WorkspaceKey, index: usize, spaces: &SpacesSettings) -> SpaceLook {
    let _stored_by = (workspace, spaces.lookup_order);
    default_look(
        index,
        Grain(u8::try_from(spaces.default_grain.0.min(100)).unwrap_or(100)),
        card_accent(spaces.default_card_accent),
    )
}

/// `spaces.default_card_accent` in quire's words.
fn card_accent(accent: DefaultCardAccent) -> CardAccent {
    match accent {
        DefaultCardAccent::Postmark => CardAccent::Postmark,
        DefaultCardAccent::SpaceHue => CardAccent::SpaceHue,
    }
}

#[cfg(test)]
mod tests {
    use super::use_space_look;
    use ds::{CardAccent, Grain, PRESETS, Theme};
    use ds_settings::Count;
    use sill_model::ids::WorkspaceKey;
    use sill_settings::{DefaultCardAccent, SpacesSettings};

    #[test]
    fn a_workspace_without_a_look_takes_its_index_preset() {
        let spaces = SpacesSettings {
            default_grain: Count(20),
            default_card_accent: DefaultCardAccent::SpaceHue,
            ..SpacesSettings::default()
        };
        // (index, preset, grain): presets 1 and 2 keep 35 and 55, the rest take the key.
        const CASES: &[(usize, usize, u8)] = &[(0, 0, 35), (1, 1, 55), (2, 2, 20), (9, 1, 55)];
        for &(index, preset, grain) in CASES {
            let look = use_space_look(WorkspaceKey(40 + index as u64), index, &spaces);
            assert_eq!(look.dots, PRESETS[preset].dots.to_vec(), "index {index}");
            assert_eq!(look.grain, Grain(grain), "index {index}");
            assert_eq!(look.theme, Theme::System, "index {index}");
            assert_eq!(look.card_accent, CardAccent::SpaceHue, "index {index}");
        }
    }
}
