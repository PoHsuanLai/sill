//! Which binding a firing carries out: the one whose keys are exactly the modifiers held.

use sill_model::desktop::MacMods;
use sill_settings::{CornerBinding, CornerBindings, CornerKey, CornerKeys};

/// The corner keys for the modifiers `mods` holds (Caps Lock and Fn are not corner keys).
pub fn keys_of(mods: MacMods) -> CornerKeys {
    [
        (MacMods::COMMAND, CornerKey::Command),
        (MacMods::OPTION, CornerKey::Option),
        (MacMods::CONTROL, CornerKey::Control),
        (MacMods::SHIFT, CornerKey::Shift),
    ]
    .into_iter()
    .filter(|(held, _)| mods.holds(*held))
    .map(|(_, key)| key)
    .collect()
}

/// The binding of `bindings` for exactly the keys `held`; none when no binding matches.
pub fn pick(bindings: &CornerBindings, held: MacMods) -> Option<&CornerBinding> {
    let held = keys_of(held);
    bindings.0.iter().find(|binding| binding.keys == held)
}

#[cfg(test)]
mod tests {
    use sill_model::desktop::MacMods;
    use sill_settings::{CornerAction, CornerBinding, CornerBindings, CornerDo};

    use super::{keys_of, pick};

    /// A binding for the keys of `mods` that does `action`.
    fn bound(mods: MacMods, action: CornerAction) -> CornerBinding {
        CornerBinding {
            keys: keys_of(mods),
            action: CornerDo::Desktop(action),
        }
    }

    #[test]
    fn the_binding_for_exactly_the_held_keys_is_picked() {
        let option_command = MacMods::OPTION.with(MacMods::COMMAND);
        let plain_and_option = CornerBindings(vec![
            bound(MacMods::NONE, CornerAction::Workspaces),
            bound(MacMods::OPTION, CornerAction::Lock),
        ]);
        let only_option = CornerBindings(vec![bound(MacMods::OPTION, CornerAction::Lock)]);
        let combo = CornerBindings(vec![bound(option_command, CornerAction::Launcher)]);
        // (held, bindings, the action chosen)
        let cases = [
            (
                MacMods::NONE,
                &plain_and_option,
                Some(CornerAction::Workspaces),
            ),
            (MacMods::OPTION, &plain_and_option, Some(CornerAction::Lock)),
            (MacMods::NONE, &only_option, None),
            (MacMods::SHIFT, &plain_and_option, None),
            (option_command, &only_option, None),
            (MacMods::OPTION, &combo, None),
            (option_command, &combo, Some(CornerAction::Launcher)),
            (
                option_command.with(MacMods::CAPS_LOCK),
                &combo,
                Some(CornerAction::Launcher),
            ),
            (MacMods::NONE, &CornerBindings(Vec::new()), None),
        ];
        for (held, bindings, want) in cases {
            let chosen = pick(bindings, held).map(|binding| binding.action.clone());
            assert_eq!(
                chosen,
                want.map(CornerDo::Desktop),
                "{held:?} over {bindings:?}"
            );
        }
    }
}
