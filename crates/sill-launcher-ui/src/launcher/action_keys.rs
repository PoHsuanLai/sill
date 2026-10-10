//! A row action's key as quire draws it: the shortcut of the shared action it is bound to. A
//! standard action shows the chord this desktop's keymap really binds it to (quire's `Shortcut`
//! is written in Mac terms and drawn through the keymap); the launcher's own actions show their
//! default chord. The panel never reads a modifier to know which row action a key means: the
//! keymap resolves the key to the action and the row that carries it runs (`pane_keys.rs`). Pure.

use chordkit::Action;
use ds::prelude::*;
use sill_launcher::{ActionKeys, OwnAction};

/// `keys` as a shortcut, when the action is bound to one.
pub fn action_shortcut(keys: &ActionKeys) -> Option<Shortcut> {
    let ActionKeys::Bound(action) = keys else {
        return None;
    };
    let shortcut = match action {
        Action::Standard(standard) => Some(Shortcut::standard(*standard)),
        own => OwnAction::of(own)
            .and_then(OwnAction::default_chord)
            .and_then(Shortcut::from_default_chord),
    };
    shortcut.filter(|shortcut| !shortcut.keys().is_empty())
}

#[cfg(test)]
mod tests {
    use super::action_shortcut;
    use chordkit::StandardAction;
    use sill_launcher::{ActionKeys, OwnAction};

    #[test]
    fn a_bound_action_draws_its_chord_in_the_macs_order() {
        let cases: Vec<(ActionKeys, Option<&str>)> = vec![
            (ActionKeys::standard(StandardAction::Reveal), Some("⌘R")),
            (ActionKeys::standard(StandardAction::Copy), Some("⌘C")),
            (ActionKeys::own(OwnAction::ForgetEntry), Some("⌘⌫")),
            (ActionKeys::own(OwnAction::CopyPath), Some("⌥⌘C")),
            (ActionKeys::own(OwnAction::SearchWeb), Some("⇧⌘B")),
            (ActionKeys::None, None),
        ];
        for (keys, want) in cases {
            let drawn = action_shortcut(&keys).map(|shortcut| shortcut.glyphs());
            assert_eq!(drawn.as_deref(), want, "{keys:?}");
        }
    }
}
