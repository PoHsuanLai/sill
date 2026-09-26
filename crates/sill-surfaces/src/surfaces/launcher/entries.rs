//! Groups as the quire `CommandPalette`'s and `Menu`'s entries (design/13 §13.3.9: a 34 px
//! tile, the title, the subtitle as its detail, a trailing hint). The selected row, which Enter
//! runs, trails `↵`, so it moves with the selection as Spotlight's return glyph does (F742); any
//! other row with actions trails `⌃K`; a row in a mixed group (Top hit, Recent) otherwise names
//! its kind. Pure.
//!
//! A row's tile is the icon file the app index resolved (`IconRef::File`) drawn at 48 px, as a
//! `Symbolic` for a freedesktop `*-symbolic` file and an `Image` otherwise; else the quire glyph
//! its `IconRef::Glyph` names; else its provider's glyph.

use std::path::Path;

use ds::{
    Availability, ExternalIcon, Icon, IconSize, IconSource, IconUrl, MenuEntry, Shortcut, Tile,
    Trail,
};
use sill_launcher::{Action, ActionId, IconRef, Item, ItemId, ProviderKind};

use crate::surfaces::launcher::chord_keys::action_shortcut;
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

/// The palette's cursor: the line its `selected` counts, whose row Enter runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Cursor(pub usize);

/// Whether Enter runs a row (it then trails `↵`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Enter {
    Runs,
    Not,
}

impl Cursor {
    /// Whether Enter runs the row on `line`.
    pub(crate) fn on(self, line: usize) -> Enter {
        match line == self.0 {
            true => Enter::Runs,
            false => Enter::Not,
        }
    }
}

/// The palette's `groups` prop with the cursor on the first line, each row's tile from
/// `tile_of` ([`tile`] draws the resolved file as it is; the panel passes `row_icon`'s, in the
/// icon style).
pub fn palette_groups(
    groups: &[Group],
    tile_of: &dyn Fn(&Item) -> Tile,
) -> Vec<(String, Vec<MenuEntry<RowRef>>)> {
    palette_groups_at(groups, tile_of, Cursor::default())
}

/// [`palette_groups`] with the palette's cursor on `cursor` (a line per row, in order): the row
/// there trails `↵`.
pub fn palette_groups_at(
    groups: &[Group],
    tile_of: &dyn Fn(&Item) -> Tile,
    cursor: Cursor,
) -> Vec<(String, Vec<MenuEntry<RowRef>>)> {
    let mut lines = 0..;
    groups
        .iter()
        .map(|group| {
            let entries = group
                .rows
                .iter()
                .zip(&mut lines)
                .map(|(item, line)| {
                    let trail = trail(item, group.title.mixes(), cursor.on(line));
                    row_entry(item, tile_of(item), trail)
                })
                .collect();
            (group.title.text().to_owned(), entries)
        })
        .collect()
}

/// The actions menu's entries for a row, each trailing its chord when it has one (H5).
pub fn action_entries(actions: &[Action]) -> Vec<MenuEntry<ActionId>> {
    actions
        .iter()
        .map(|action| MenuEntry::Item {
            value: action.id.clone(),
            title: action.label.clone(),
            detail: None,
            tile: None,
            trail: action_shortcut(&action.keys).map_or(Trail::None, Trail::Shortcut),
            check: None,
            availability: Availability::Enabled,
        })
        .collect()
}

fn row_entry(item: &Item, tile: Tile, trail: Trail) -> MenuEntry<RowRef> {
    MenuEntry::Item {
        value: RowRef::of(item),
        title: item.title.clone(),
        detail: (!item.subtitle.is_empty()).then(|| item.subtitle.clone()),
        tile: Some(tile),
        trail,
        check: None,
        availability: Availability::Enabled,
    }
}

/// `↵` on the row Enter runs, `⌃K` where there are actions, else a mixed group's kind note.
fn trail(item: &Item, mix: Mix, enter: Enter) -> Trail {
    match (enter, item.actions.is_empty(), mix) {
        (Enter::Runs, _, _) => Trail::Shortcut(Shortcut(vec![ds::Key::Enter])),
        (Enter::Not, false, _) => {
            Trail::Shortcut(Shortcut(vec![ds::Key::Ctrl, ds::Key::Char('k')]))
        }
        (Enter::Not, true, Mix::Mixed) => Trail::Note(kind_note(item.provider).to_owned()),
        (Enter::Not, true, Mix::Single) => Trail::None,
    }
}

/// A row's kind, trailing it in a mixed group.
pub(crate) fn kind_note(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Apps => "Application",
        ProviderKind::Windows => "Window",
        ProviderKind::Calculator => "Calculator",
        ProviderKind::Settings => "Setting",
        ProviderKind::System => "System",
        ProviderKind::Files => "Document",
        ProviderKind::Clipboard => "Clipboard",
        ProviderKind::Emoji => "Emoji",
        ProviderKind::Web => "Web",
    }
}

/// The row's tile: its resolved icon file, else its glyph.
pub fn tile(item: &Item) -> Tile {
    match &item.icon {
        IconRef::File(path) => file_icon(path).map_or(Tile::Icon(glyph(item)), Tile::Source),
        IconRef::Glyph(_) | IconRef::None => Tile::Icon(glyph(item)),
    }
}

/// An icon file at the rows' 48 px: `Symbolic` for a `*-symbolic` file, recoloured like a
/// glyph, `Image` for the rest; none for a relative path.
fn file_icon(path: &Path) -> Option<IconSource> {
    let url = IconUrl::file(path).ok()?;
    let external = ExternalIcon {
        url,
        size: IconSize::Tile48,
    };
    let symbolic = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.ends_with("-symbolic"));
    Some(if symbolic {
        IconSource::Symbolic(external)
    } else {
        IconSource::Image(external)
    })
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
        // M9: quire's glyphs for the new providers (Q296).
        ProviderKind::Files => Icon::File,
        ProviderKind::Clipboard => Icon::Clipboard,
        ProviderKind::Emoji => Icon::Smile,
        ProviderKind::Web => Icon::Globe,
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
    use super::{Cursor, RowRef, glyph, kebab, named, palette_groups, palette_groups_at, tile};
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
                    keys: sill_launcher::ActionKeys::None,
                })
                .collect(),
            subject: sill_launcher::Subject::None,
        }
    }

    fn trails(query: &str, items: &[Item]) -> Vec<(String, Trail)> {
        palette_groups(&groups(items, query, Filter::All), &tile)
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

    /// F742: `↵` is on the row the cursor is on (Enter runs it), and the first row, no longer
    /// selected, trails its `⌃K` like the rest.
    #[test]
    fn the_return_glyph_moves_with_the_selection() {
        let items = vec![
            item(ProviderKind::Apps, "Firefox", 2),
            item(ProviderKind::Apps, "Files", 1),
            item(ProviderKind::Windows, "Mail", 0),
        ];
        let enter = Trail::Shortcut(Shortcut(vec![ds::Key::Enter]));
        let ctrl_k = Trail::Shortcut(Shortcut(vec![ds::Key::Ctrl, ds::Key::Char('k')]));
        let at = |line: usize| -> Vec<Trail> {
            palette_groups_at(&groups(&items, "f", Filter::All), &tile, Cursor(line))
                .into_iter()
                .flat_map(|(_, entries)| entries)
                .filter_map(|entry| match entry {
                    MenuEntry::Item { trail, .. } => Some(trail),
                    _ => None,
                })
                .collect()
        };
        let cases: Vec<(usize, Vec<Trail>)> = vec![
            (0, vec![enter.clone(), ctrl_k.clone(), Trail::None]),
            (1, vec![ctrl_k.clone(), enter.clone(), Trail::None]),
            (2, vec![ctrl_k.clone(), ctrl_k.clone(), enter.clone()]),
            // Past the last row (no such line): no row trails `↵`.
            (9, vec![ctrl_k.clone(), ctrl_k, Trail::None]),
        ];
        for (line, want) in cases {
            assert_eq!(at(line), want, "cursor on line {line}");
        }
    }

    #[test]
    fn rows_carry_their_identity_detail_and_glyph() {
        let items = vec![item(ProviderKind::System, "Lock", 0)];
        let got = palette_groups(&groups(&items, "l", Filter::All), &tile);
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
        assert_eq!(glyph(&calc), Icon::Command, "a file's fallback glyph");
    }

    #[test]
    fn a_resolved_icon_file_is_the_rows_tile() {
        use ds::{IconSize, IconSource, Tile};
        let mut app = item(ProviderKind::Apps, "Firefox", 0);
        let word = |tile: Tile| match tile {
            Tile::Source(IconSource::Image(e)) => format!("image {} {:?}", e.url.as_str(), e.size),
            Tile::Source(IconSource::Symbolic(e)) => format!("symbolic {}", e.url.as_str()),
            Tile::Icon(icon) => format!("glyph {icon:?}"),
            other => format!("{other:?}"),
        };
        assert_eq!(
            word(tile(&app)),
            "glyph Grid",
            "no icon: the provider's glyph"
        );
        app.icon = IconRef::File("/usr/share/icons/firefox.png".into());
        assert_eq!(
            word(tile(&app)),
            format!(
                "image file:///usr/share/icons/firefox.png {:?}",
                IconSize::Tile48
            )
        );
        app.icon = IconRef::File("/i/term-symbolic.svg".into());
        assert_eq!(word(tile(&app)), "symbolic file:///i/term-symbolic.svg");
        app.icon = IconRef::File("relative.png".into());
        assert_eq!(word(tile(&app)), "glyph Grid", "a relative path is refused");
        app.icon = IconRef::Glyph("power".to_owned());
        assert_eq!(word(tile(&app)), "glyph Power");
    }
}
