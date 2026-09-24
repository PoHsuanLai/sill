//! Groups as the quire `CommandPalette`'s and `Menu`'s entries (design/13 §13.3.9: a 34 px
//! tile, the title, the subtitle as its detail, a trailing hint). The first row, which Enter
//! runs, trails `↵`; any other row with actions trails `⌃K`; a row in a mixed group (Top hit,
//! Recent) otherwise names its kind. Pure.
//!
//! A `Tile` is a quire glyph only, so an app's own icon file cannot be drawn here: every row
//! shows its `IconRef::Glyph` when it names a quire icon, else its provider's glyph (Q42).

use ds::{Availability, Icon, MenuEntry, Shortcut, Tile, Trail};
use sill_launcher::{Action, ActionId, IconRef, Item, ItemId, ProviderKind};

use crate::surfaces::launcher::groups::{Group, Mix};

/// What a palette row picks: the row's identity, looked up again when it is chosen so a
/// batch landing between the key and the pick cannot run another row.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RowRef {
    pub provider: ProviderKind,
    pub id: ItemId,
}

impl RowRef {
    /// The reference to `item`.
    pub fn of(item: &Item) -> RowRef {
        RowRef {
            provider: item.provider,
            id: item.id.clone(),
        }
    }

    /// The row this refers to among `items`.
    pub fn find<'a>(&self, items: &'a [Item]) -> Option<&'a Item> {
        items
            .iter()
            .find(|item| item.provider == self.provider && item.id == self.id)
    }
}

/// The palette's `groups` prop.
pub fn palette_groups(groups: &[Group]) -> Vec<(String, Vec<MenuEntry<RowRef>>)> {
    let mut first = Some(());
    groups
        .iter()
        .map(|group| {
            let entries = group
                .rows
                .iter()
                .map(|item| row_entry(item, trail(item, group.title.mixes(), first.take())))
                .collect();
            (group.title.text().to_owned(), entries)
        })
        .collect()
}

/// The actions menu's entries for a row.
pub fn action_entries(actions: &[Action]) -> Vec<MenuEntry<ActionId>> {
    actions
        .iter()
        .map(|action| MenuEntry::Item {
            value: action.id.clone(),
            title: action.label.clone(),
            detail: None,
            tile: None,
            trail: Trail::None,
            check: None,
            availability: Availability::Enabled,
        })
        .collect()
}

fn row_entry(item: &Item, trail: Trail) -> MenuEntry<RowRef> {
    MenuEntry::Item {
        value: RowRef::of(item),
        title: item.title.clone(),
        detail: (!item.subtitle.is_empty()).then(|| item.subtitle.clone()),
        tile: Some(Tile::Icon(glyph(item))),
        trail,
        check: None,
        availability: Availability::Enabled,
    }
}

/// `↵` on the row Enter runs, `⌃K` where there are actions, else a mixed group's kind note.
fn trail(item: &Item, mix: Mix, first: Option<()>) -> Trail {
    match (first, item.actions.is_empty(), mix) {
        (Some(()), _, _) => Trail::Shortcut(Shortcut(vec![ds::Key::Enter])),
        (None, false, _) => Trail::Shortcut(Shortcut(vec![ds::Key::Ctrl, ds::Key::Char('k')])),
        (None, true, Mix::Mixed) => Trail::Note(kind_note(item.provider).to_owned()),
        (None, true, Mix::Single) => Trail::None,
    }
}

/// A row's kind, trailing it in a mixed group.
fn kind_note(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Apps => "Application",
        ProviderKind::Windows => "Window",
        ProviderKind::Calculator => "Calculator",
        ProviderKind::Settings => "Setting",
        ProviderKind::System => "System",
    }
}

/// The row's glyph: the quire icon it names, else its provider's.
pub fn glyph(item: &Item) -> Icon {
    match &item.icon {
        IconRef::Glyph(name) => named(name).unwrap_or(provider_glyph(item.provider)),
        IconRef::File(_) | IconRef::None => provider_glyph(item.provider),
    }
}

/// Each provider's glyph (design/08: `Grid` for applications, `Window` for windows).
pub fn provider_glyph(kind: ProviderKind) -> Icon {
    match kind {
        ProviderKind::Apps => Icon::Grid,
        ProviderKind::Windows => Icon::Window,
        ProviderKind::Calculator => Icon::Command,
        ProviderKind::Settings => Icon::Settings,
        ProviderKind::System => Icon::Power,
    }
}

/// The quire icon whose kebab-case name is `name` (`power`, `wifi-low`), as `IconRef::Glyph`
/// carries it. quire has no name lookup, so the variant's own name is kebab-cased.
pub fn named(name: &str) -> Option<Icon> {
    Icon::ALL
        .iter()
        .copied()
        .find(|icon| kebab(&format!("{icon:?}")) == name)
}

fn kebab(camel: &str) -> String {
    camel
        .chars()
        .enumerate()
        .flat_map(|(i, c)| {
            let dash = (i > 0 && (c.is_ascii_uppercase() || c.is_ascii_digit())).then_some('-');
            dash.into_iter().chain(c.to_lowercase())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{RowRef, glyph, kebab, named, palette_groups};
    use crate::surfaces::launcher::groups::groups;
    use crate::surfaces::launcher::keys::Filter;
    use ds::{Icon, MenuEntry, Shortcut, Trail};
    use sill_launcher::{Action, ActionId, IconRef, Item, ItemId, ProviderKind, Score};

    fn item(provider: ProviderKind, title: &str, actions: usize) -> Item {
        Item {
            id: ItemId(title.to_owned()),
            provider,
            title: title.to_owned(),
            subtitle: format!("{title} sub"),
            icon: IconRef::None,
            score: Score(1),
            actions: (0..actions)
                .map(|i| Action {
                    id: ActionId(format!("a{i}")),
                    label: format!("Action {i}"),
                })
                .collect(),
        }
    }

    fn trails(query: &str, items: &[Item]) -> Vec<(String, Trail)> {
        palette_groups(&groups(items, query, Filter::All))
            .into_iter()
            .flat_map(|(_, entries)| entries)
            .filter_map(|entry| match entry {
                MenuEntry::Item { title, trail, .. } => Some((title, trail)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn hints_mark_enter_actions_and_kinds() {
        let items = vec![
            item(ProviderKind::Apps, "Firefox", 2),
            item(ProviderKind::Apps, "Files", 1),
            item(ProviderKind::Windows, "Mail", 0),
        ];
        let enter = Trail::Shortcut(Shortcut(vec![ds::Key::Enter]));
        let ctrl_k = Trail::Shortcut(Shortcut(vec![ds::Key::Ctrl, ds::Key::Char('k')]));
        assert_eq!(
            trails("f", &items),
            vec![
                ("Firefox".to_owned(), enter.clone()),
                ("Files".to_owned(), ctrl_k),
                ("Mail".to_owned(), Trail::None),
            ]
        );
        // Recent mixes providers: a row without actions names its kind.
        let recent = trails("", &items);
        assert_eq!(recent[0].1, enter);
        assert_eq!(
            recent[2],
            ("Mail".to_owned(), Trail::Note("Window".to_owned()))
        );
    }

    #[test]
    fn rows_carry_their_identity_detail_and_glyph() {
        let items = vec![item(ProviderKind::System, "Lock", 0)];
        let got = palette_groups(&groups(&items, "l", Filter::All));
        assert_eq!(got[0].0, "Top hit");
        match &got[0].1[0] {
            MenuEntry::Item {
                value,
                detail,
                tile,
                ..
            } => {
                assert_eq!(value, &RowRef::of(&items[0]));
                assert_eq!(detail.as_deref(), Some("Lock sub"));
                assert_eq!(tile, &Some(ds::Tile::Icon(Icon::Power)));
            }
            other => panic!("not an item: {other:?}"),
        }
    }

    #[test]
    fn glyph_names_are_quires_in_kebab_case() {
        assert_eq!(kebab("WifiLow"), "wifi-low");
        assert_eq!(kebab("Volume2"), "volume-2");
        assert_eq!(named("power"), Some(Icon::Power));
        assert_eq!(named("wifi-low"), Some(Icon::WifiLow));
        assert_eq!(named("no-such-icon"), None);
        let mut calc = item(ProviderKind::Calculator, "= 4", 0);
        assert_eq!(glyph(&calc), Icon::Command);
        calc.icon = IconRef::Glyph("copy".to_owned());
        assert_eq!(glyph(&calc), Icon::Copy);
        calc.icon = IconRef::File("/usr/share/icons/x.svg".into());
        assert_eq!(
            glyph(&calc),
            Icon::Command,
            "no file icons in a palette tile (Q42)"
        );
    }
}
