//! Keys for row actions, written in Mac terms (quire design/27 §6.2 rule (e): bindings are Mac
//! chords, the platform layer resolves Command; Toshy maps it to Ctrl). A row's action carries its
//! chord so the results run it without opening the actions menu, and the menu and the preview pane
//! show it beside the action (quire design/09 H5, "fewer icons, more shortcuts"). Pure.

use serde::{Deserialize, Serialize};

use crate::ids::ActionId;
use crate::item::Item;

/// An action's chord, or none.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ActionKeys {
    #[default]
    None,
    Chord(Chord),
}

/// Modifiers and a key. `mods` is kept in the Mac's order, ⌃⌥⇧⌘ ([`Chord::new`] sorts it).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Chord {
    pub mods: Vec<Modifier>,
    pub key: ChordKey,
}

/// A modifier, in the order the Mac lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Control,
    Option,
    Shift,
    Command,
}

/// The key a chord ends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ChordKey {
    Enter,
    Space,
    Left,
    Right,
    Backspace,
    /// A letter or digit, lower case.
    Char(char),
}

impl Chord {
    /// `key` with `mods`, sorted and deduplicated into ⌃⌥⇧⌘ order.
    pub fn new(mut mods: Vec<Modifier>, key: ChordKey) -> Chord {
        mods.sort();
        mods.dedup();
        Chord { mods, key }
    }

    /// ⌘ and `key`.
    pub fn command(key: ChordKey) -> Chord {
        Chord::new(vec![Modifier::Command], key)
    }
}

/// The row actions' chords (Spotlight's where it has one).
pub mod chords {
    use super::{Chord, ChordKey};

    /// Reveal in Files: ⌘R (Spotlight's "Show in Finder" is ⌘R and ⌘↵).
    pub fn reveal() -> Chord {
        Chord::command(ChordKey::Char('r'))
    }

    /// Copy (a path, an emoji, a clipboard entry's text): ⌘C.
    pub fn copy() -> Chord {
        Chord::command(ChordKey::Char('c'))
    }

    /// Search the web for the query: ⌘B (Spotlight's).
    pub fn search_web() -> Chord {
        Chord::command(ChordKey::Char('b'))
    }

    /// Toggle the preview pane: ⌘Y (Quick Look's standard key).
    pub fn quick_look() -> Chord {
        Chord::command(ChordKey::Char('y'))
    }

    /// Forget a clipboard entry: ⌘⌫.
    pub fn forget() -> Chord {
        Chord::command(ChordKey::Backspace)
    }
}

/// The action of `item` whose chord is `chord`, if any.
pub fn action_for_chord<'a>(item: &'a Item, chord: &Chord) -> Option<&'a ActionId> {
    item.actions
        .iter()
        .find(|action| action.keys == ActionKeys::Chord(chord.clone()))
        .map(|action| &action.id)
}

#[cfg(test)]
mod tests {
    use super::{ActionKeys, Chord, ChordKey, Modifier, action_for_chord, chords};
    use crate::ids::{ActionId, ItemId, ProviderKind};
    use crate::item::{Action, IconRef, Item};
    use crate::rank::Score;
    use crate::subject::Subject;

    #[test]
    fn modifiers_sort_into_the_macs_order() {
        let chord = Chord::new(
            vec![
                Modifier::Command,
                Modifier::Shift,
                Modifier::Control,
                Modifier::Command,
            ],
            ChordKey::Char('k'),
        );
        assert_eq!(
            chord.mods,
            [Modifier::Control, Modifier::Shift, Modifier::Command]
        );
    }

    #[test]
    fn a_chord_finds_the_action_that_carries_it() {
        let item = Item {
            id: ItemId("f".to_owned()),
            provider: ProviderKind::Files,
            title: "f".to_owned(),
            subtitle: String::new(),
            icon: IconRef::None,
            score: Score(0),
            actions: vec![
                Action {
                    id: ActionId("reveal".to_owned()),
                    label: "Reveal in Files".to_owned(),
                    keys: ActionKeys::Chord(chords::reveal()),
                },
                Action {
                    id: ActionId("plain".to_owned()),
                    label: "Plain".to_owned(),
                    keys: ActionKeys::None,
                },
            ],
            subject: Subject::None,
        };
        assert_eq!(
            action_for_chord(&item, &chords::reveal()),
            Some(&ActionId("reveal".to_owned()))
        );
        assert_eq!(action_for_chord(&item, &chords::copy()), None);
    }
}
