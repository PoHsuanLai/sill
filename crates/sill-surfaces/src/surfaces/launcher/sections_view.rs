//! The published sections as the palette's groups (M9 lane d, FINDINGS "M9 freeze" F645,
//! F656): `sill_launcher::shown` decides which rows each section shows; this names them for
//! quire's `CommandPalette` and says what each header's trailing action is ("Show More" or
//! "Show Less", quire's `SectionHeader` action: Q294).
//!
//! Two swap points wait on quire's m9-palette: a file or clipboard row is a plain
//! `MenuEntry::Item` until `RowShape::File`/`RowShape::Clip` (Q290), and a header's action is a
//! stand-in line after the section's rows until `PaletteGroup { action }` (Q294).

use ds::{Availability, Icon, MenuEntry, Shortcut, Tile, Trail};
use sill_launcher::{Item, More, Section, ShownSection, UnixTime};

use crate::surfaces::launcher::chord_keys::action_shortcut;
use crate::surfaces::launcher::entries::{RowRef, kind_note};
use crate::surfaces::launcher::groups::Mix;
use crate::surfaces::launcher::rows::{RowDetail, row_detail};
use crate::surfaces::launcher::view::more_ref;

/// A section header's trailing action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeaderAction {
    None,
    ShowMore,
    ShowLess,
}

impl HeaderAction {
    /// The action for a section's `more`.
    pub fn of(more: More) -> HeaderAction {
        match more {
            More::Nothing => HeaderAction::None,
            More::Hidden(_) => HeaderAction::ShowMore,
            More::Expanded => HeaderAction::ShowLess,
        }
    }

    /// Its words.
    pub fn label(self) -> Option<&'static str> {
        match self {
            HeaderAction::None => None,
            HeaderAction::ShowMore => Some("Show More"),
            HeaderAction::ShowLess => Some("Show Less"),
        }
    }
}

/// `shown` as the palette's groups (title, rows), each row's tile from `tile_of`, the first
/// row trailing `↵` and rows with actions their first chord or `⌃K` (the M4 rule, `entries.rs`),
/// file and clipboard rows their `rows::row_detail` trail. The emoji section is left out when
/// the grid draws it (not yet: quire's `EmojiGrid`, Q291, so emoji are rows for now).
pub fn palette_sections(
    shown: &[ShownSection],
    tile_of: &dyn Fn(&Item) -> Tile,
) -> Vec<(String, Vec<MenuEntry<RowRef>>)> {
    sections_at(shown, tile_of, now())
}

/// [`palette_sections`] at `now` (tests pin the clock).
pub(crate) fn sections_at(
    shown: &[ShownSection],
    tile_of: &dyn Fn(&Item) -> Tile,
    now: UnixTime,
) -> Vec<(String, Vec<MenuEntry<RowRef>>)> {
    let mut first = Some(());
    shown
        .iter()
        .map(|section| {
            let mix = mix_of(section.section);
            let mut entries: Vec<MenuEntry<RowRef>> = section
                .rows
                .iter()
                .map(|item| {
                    let words = row_detail(item, now);
                    let trail = trail(item, &words, mix, first.take());
                    row_entry(item, tile_of(item), words.detail, trail)
                })
                .collect();
            entries.extend(more_entry(section));
            (section.section.title().to_owned(), entries)
        })
        .collect()
}

/// Whether a section mixes providers, so a row's kind is worth a note.
fn mix_of(section: Section) -> Mix {
    match section {
        Section::TopHit | Section::Recent => Mix::Mixed,
        _ => Mix::Single,
    }
}

/// `↵` on the row Enter runs; a file's date or a clipboard entry's age; else the first chord of
/// its actions, `⌃K` when none has one; else, in a mixed section, its kind.
fn trail(item: &Item, words: &RowDetail, mix: Mix, first: Option<()>) -> Trail {
    let chord = item
        .actions
        .iter()
        .find_map(|action| action_shortcut(&action.keys));
    if first.is_some() {
        return Trail::Shortcut(Shortcut(vec![ds::Key::Enter]));
    }
    if !words.trail.is_empty() {
        return Trail::Note(words.trail.clone());
    }
    match (chord, item.actions.is_empty(), mix) {
        (Some(chord), _, _) => Trail::Shortcut(chord),
        (None, false, _) => Trail::Shortcut(Shortcut(vec![ds::Key::Ctrl, ds::Key::Char('k')])),
        (None, true, Mix::Mixed) => Trail::Note(kind_note(item.provider).to_owned()),
        (None, true, Mix::Single) => Trail::None,
    }
}

// Swap point (Q290): a file or clipboard row becomes quire's `RowShape::File`/`RowShape::Clip`
// once the palette has them; until then every row is a plain `MenuEntry::Item`.
fn row_entry(item: &Item, tile: Tile, detail: String, trail: Trail) -> MenuEntry<RowRef> {
    MenuEntry::Item {
        value: RowRef::of(item),
        title: item.title.clone(),
        detail: (!detail.is_empty()).then_some(detail),
        tile: Some(tile),
        trail,
        check: None,
        availability: Availability::Enabled,
    }
}

// Swap point (Q294): the header's trailing action replaces this stand-in line once the palette
// has `PaletteGroup { action }`.
fn more_entry(section: &ShownSection) -> Option<MenuEntry<RowRef>> {
    let label = HeaderAction::of(section.more).label()?;
    let value = more_ref(section.section)?;
    let (icon, detail) = match section.more {
        More::Hidden(n) => (Icon::ChevronDown, Some(format!("{n} more"))),
        More::Nothing | More::Expanded => (Icon::ChevronUp, None),
    };
    Some(MenuEntry::Item {
        value,
        title: label.to_owned(),
        detail,
        tile: Some(Tile::Icon(icon)),
        trail: Trail::None,
        check: None,
        availability: Availability::Enabled,
    })
}

fn now() -> UnixTime {
    UnixTime(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_secs()),
    )
}

#[cfg(test)]
mod tests {
    use ds::{MenuEntry, Shortcut, Tile, Trail};
    use sill_launcher::{
        Action, ActionId, ActionKeys, IconRef, Item, ItemId, More, ProviderKind, Score, Section,
        ShownSection, Subject, UnixTime, chords,
    };

    use super::{HeaderAction, sections_at};
    use crate::surfaces::launcher::chord_keys::shortcut;
    use crate::surfaces::launcher::entries::tile;
    use crate::surfaces::launcher::view::more_ref;

    #[test]
    fn a_capped_section_offers_more_and_an_expanded_one_less() {
        assert_eq!(HeaderAction::of(More::Hidden(3)).label(), Some("Show More"));
        assert_eq!(HeaderAction::of(More::Expanded).label(), Some("Show Less"));
        assert_eq!(HeaderAction::of(More::Nothing).label(), None);
    }

    fn item(provider: ProviderKind, title: &str, actions: Vec<Action>) -> Item {
        Item {
            id: ItemId(title.to_owned()),
            provider,
            title: title.to_owned(),
            subtitle: String::new(),
            icon: IconRef::None,
            score: Score(1),
            actions,
            subject: Subject::None,
        }
    }

    fn entries(shown: &[ShownSection]) -> Vec<(String, Vec<(String, Trail)>)> {
        sections_at(shown, &tile, UnixTime(0))
            .into_iter()
            .map(|(title, entries)| {
                let rows = entries
                    .into_iter()
                    .filter_map(|entry| match entry {
                        MenuEntry::Item { title, trail, .. } => Some((title, trail)),
                        _ => None,
                    })
                    .collect();
                (title, rows)
            })
            .collect()
    }

    #[test]
    fn sections_trail_enter_chords_and_a_show_more_line() {
        let reveal = Action {
            id: ActionId("reveal".to_owned()),
            label: "Reveal in Files".to_owned(),
            keys: ActionKeys::Chord(chords::reveal()),
        };
        let plain = Action {
            id: ActionId("new".to_owned()),
            label: "New Window".to_owned(),
            keys: ActionKeys::None,
        };
        let shown = vec![
            ShownSection {
                section: Section::TopHit,
                rows: vec![item(ProviderKind::Apps, "Files", vec![])],
                more: More::Nothing,
            },
            ShownSection {
                section: Section::Documents,
                rows: vec![
                    item(ProviderKind::Files, "a.pdf", vec![reveal]),
                    item(ProviderKind::Files, "b.pdf", vec![plain]),
                ],
                more: More::Hidden(4),
            },
        ];
        let enter = Trail::Shortcut(Shortcut(vec![ds::Key::Enter]));
        let ctrl_k = Trail::Shortcut(Shortcut(vec![ds::Key::Ctrl, ds::Key::Char('k')]));
        assert_eq!(
            entries(&shown),
            vec![
                ("Top Hit".to_owned(), vec![("Files".to_owned(), enter)]),
                (
                    "Documents".to_owned(),
                    vec![
                        (
                            "a.pdf".to_owned(),
                            Trail::Shortcut(shortcut(&chords::reveal()))
                        ),
                        ("b.pdf".to_owned(), ctrl_k),
                        ("Show More".to_owned(), Trail::None),
                    ]
                ),
            ]
        );
        let got = sections_at(&shown, &tile, UnixTime(0));
        match &got[1].1[2] {
            MenuEntry::Item {
                value, detail, tile, ..
            } => {
                assert_eq!(Some(value.clone()), more_ref(Section::Documents));
                assert_eq!(detail.as_deref(), Some("4 more"));
                assert_eq!(tile, &Some(Tile::Icon(ds::Icon::ChevronDown)));
            }
            other => panic!("not an item: {other:?}"),
        }
    }
}
