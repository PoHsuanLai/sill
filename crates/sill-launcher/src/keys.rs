//! The keys of a row's actions, as the shared keymap names them. An action is bound to a chordkit
//! [`Action`]: a standard one (Copy, Reveal, Quick Look, Open) whose chord the desktop decides, or
//! one of the launcher's own ([`OwnAction`]) whose portable default chord is written here and which
//! the panel registers once. The launcher never reads a modifier and never draws a chord of its
//! own: the keymap resolves the key and quire draws the shortcut. Pure.

use chordkit::{Action, AppAction, AppId, DefaultChord, Registration, StandardAction};
use ds_core::word::Word;
use serde::{Deserialize, Serialize};

use crate::ids::ActionId;
use crate::item::Item;

/// What a row action is bound to, or nothing.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum ActionKeys {
    #[default]
    None,
    Bound(Action),
}

impl ActionKeys {
    /// Bound to the standard `action`.
    pub fn standard(action: StandardAction) -> ActionKeys {
        ActionKeys::Bound(Action::Standard(action))
    }

    /// Bound to one of the launcher's own actions.
    pub fn own(action: OwnAction) -> ActionKeys {
        action
            .action()
            .map_or(ActionKeys::None, |own| ActionKeys::Bound(Action::App(own)))
    }
}

/// An action only the launcher has. Its id is `sill.` and its slug.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Word)]
pub enum OwnAction {
    /// Search the web for the query.
    SearchWeb,
    /// Forget a clipboard entry.
    ForgetEntry,
    /// Copy a file's path.
    CopyPath,
    /// Open the selected row's actions.
    ShowActions,
    /// Move the keyboard into the preview pane.
    FocusPane,
    /// Send the companion's question or answer on.
    Promote,
    /// Switch Permissions Mode: flip the conversation between Auto and Manual.
    SwitchMode,
}

impl OwnAction {
    /// The shared action, or none for an id the keymap would refuse (never for these).
    pub fn action(self) -> Option<AppAction> {
        AppAction::new(&format!("{APP}.{}", self.slug())).ok()
    }

    /// The portable chord it starts with. `Primary` is Command on our desktop. None of these is
    /// a standard action's chord, which the keymap would refuse: the launcher offers no bold and
    /// no Move to Trash, so [`registration`] forgoes both; Search the web keeps Spotlight's ⌘B
    /// and forgetting a clipboard entry keeps ⌘⌫.
    pub fn default_chord(self) -> Option<DefaultChord> {
        let text = match self {
            OwnAction::SearchWeb => "Primary+B",
            OwnAction::ForgetEntry => "Primary+Backspace",
            OwnAction::CopyPath => "Primary+Alt+C",
            OwnAction::ShowActions => "Primary+K",
            OwnAction::FocusPane => "Primary+Right",
            OwnAction::Promote => "Primary+Enter",
            OwnAction::SwitchMode => "Primary+Shift+M",
        };
        text.parse().ok()
    }

    /// The own action `action` is, when it is one.
    pub fn of(action: &Action) -> Option<OwnAction> {
        match action {
            Action::App(app) => OwnAction::ALL
                .iter()
                .copied()
                .find(|own| own.action().as_ref() == Some(app)),
            _ => None,
        }
    }
}

/// The app name the launcher's own actions are registered under.
pub const APP: &str = "sill";

/// What the panel registers for `app`: every own action with its default chord, and the standard
/// actions the launcher does not offer (Bold, whose ⌘B Search the web takes, and Move to Trash,
/// whose ⌘⌫ forgets a clipboard entry).
pub fn registration(app: &AppId) -> Registration {
    let rows = OwnAction::ALL
        .iter()
        .filter_map(|own| Some((own.action()?, own.default_chord()?)));
    rows.fold(
        Registration::new(app)
            .forgo(StandardAction::Bold)
            .forgo(StandardAction::MoveToTrash),
        |registration, (action, chord)| registration.action(action, chord),
    )
}

/// The action of `item` bound to `action`, if any.
pub fn action_for_key<'a>(item: &'a Item, action: &Action) -> Option<&'a ActionId> {
    item.actions
        .iter()
        .find(|row| row.keys == ActionKeys::Bound(action.clone()))
        .map(|row| &row.id)
}

#[cfg(test)]
mod tests {
    use super::{ActionKeys, OwnAction, action_for_key, registration};
    use crate::ids::{ActionId, ItemId, ProviderKind};
    use crate::item::{Action, IconRef, Item};
    use crate::rank::Score;
    use crate::subject::Subject;
    use chordkit::{Action as Bound, AppId, Keymap, Platform, StandardAction};
    use ds_core::word::Word;

    #[test]
    fn every_own_action_registers_on_our_desktop_without_a_conflict() {
        let ours = Platform::Linux {
            desktop: chordkit::Desktop::Ours,
        };
        let app = AppId::new(super::APP).unwrap_or_else(|e| panic!("{e}"));
        let mut keymap = Keymap::conventional(ours);
        assert_eq!(keymap.register_with(&registration(&app)), Ok(()));
        for own in OwnAction::ALL {
            let action = own.action().map(Bound::App);
            let bound = action.is_some_and(|action| !keymap.chords_of(&action).is_empty());
            assert!(bound, "{own:?} has a chord");
        }
    }

    #[test]
    fn an_own_action_is_found_again_from_its_shared_action() {
        for own in OwnAction::ALL {
            let action = own.action().map(Bound::App);
            assert_eq!(
                action.as_ref().and_then(OwnAction::of),
                Some(*own),
                "{own:?}"
            );
        }
        assert_eq!(OwnAction::of(&Bound::Standard(StandardAction::Copy)), None);
    }

    #[test]
    fn a_key_finds_the_row_action_bound_to_it() {
        let action = |id: &str, keys| Action {
            id: ActionId(id.to_owned()),
            label: id.to_owned(),
            keys,
        };
        let item = Item {
            id: ItemId("f".to_owned()),
            provider: ProviderKind::Files,
            title: "f".to_owned(),
            subtitle: String::new(),
            icon: IconRef::None,
            score: Score(0),
            actions: vec![
                action("reveal", ActionKeys::standard(StandardAction::Reveal)),
                action("plain", ActionKeys::None),
            ],
            subject: Subject::None,
        };
        let reveal = Bound::Standard(StandardAction::Reveal);
        let copy = Bound::Standard(StandardAction::Copy);
        assert_eq!(
            action_for_key(&item, &reveal),
            Some(&ActionId("reveal".to_owned()))
        );
        assert_eq!(action_for_key(&item, &copy), None);
    }
}
