use std::path::PathBuf;

use dioxus::prelude::*;
use ds::{ChordShown, ClipBody, GroupEntries, MenuEntry, RowChord, RowShape, Trail};
use sill_launcher::{
    Action, ActionId, ActionKeys, CharCount, ClipFacts, ClipKey, ClipKind, EmojiFacts, Excerpt,
    FileFacts, IconRef, Item, ItemId, Mime, More, ProviderKind, Score, Section, ShownSection,
    SkinTone, Subject, UnixTime, chords,
};

use super::{Draw, HeaderAction, palette_groups, palette_sections};
use crate::surfaces::launcher::chord_keys::shortcut;
use crate::surfaces::launcher::entries::tile;

const NOW: UnixTime = UnixTime(1_790_000_000);

#[test]
fn a_capped_section_offers_more_and_an_expanded_one_less() {
    assert_eq!(HeaderAction::of(More::Hidden(3)).label(), Some("Show More"));
    assert_eq!(HeaderAction::of(More::Expanded).label(), Some("Show Less"));
    assert_eq!(HeaderAction::of(More::Nothing).label(), None);
}

fn item(provider: ProviderKind, title: &str, subject: Subject) -> Item {
    Item {
        id: ItemId(title.to_owned()),
        provider,
        title: title.to_owned(),
        subtitle: "~/Documents".to_owned(),
        icon: IconRef::None,
        score: Score(1),
        actions: vec![Action {
            id: ActionId("reveal".to_owned()),
            label: "Reveal in Files".to_owned(),
            keys: ActionKeys::Chord(chords::reveal()),
        }],
        subject,
    }
}

fn file(name: &str) -> Subject {
    Subject::File(FileFacts {
        path: PathBuf::from(format!("/h/Documents/{name}")),
        mime: Mime("application/pdf".to_owned()),
        modified: UnixTime(NOW.0 - 86_400),
    })
}

fn sample() -> Vec<ShownSection> {
    vec![
        ShownSection {
            section: Section::TopHit,
            rows: vec![item(ProviderKind::Files, "a.pdf", file("a.pdf"))],
            more: More::Nothing,
        },
        ShownSection {
            section: Section::Documents,
            rows: vec![item(ProviderKind::Files, "b.pdf", file("b.pdf"))],
            more: More::Hidden(4),
        },
        ShownSection {
            section: Section::Clipboard,
            rows: vec![item(
                ProviderKind::Clipboard,
                "hello",
                Subject::Clip(ClipFacts {
                    key: ClipKey(3),
                    kind: ClipKind::Text {
                        excerpt: Excerpt("hello".to_owned()),
                        chars: CharCount(5),
                    },
                    copied: UnixTime(NOW.0 - 120),
                }),
            )],
            more: More::Nothing,
        },
        ShownSection {
            section: Section::Emoji,
            rows: vec![item(
                ProviderKind::Emoji,
                "🎉",
                Subject::Emoji(EmojiFacts {
                    glyph: "🎉".to_owned(),
                    name: "party popper".to_owned(),
                    tone: SkinTone::Default,
                }),
            )],
            more: More::Expanded,
        },
    ]
}

#[test]
fn groups_carry_shapes_a_grid_and_header_actions() {
    let draw = Draw {
        tile_of: &tile,
        clip_image: &|_| None,
        now: NOW,
    };
    // A handler needs a Dioxus runtime to be made in.
    let mut dom = VirtualDom::new(|| rsx! {});
    dom.rebuild_in_place();
    let groups = dom.in_scope(ScopeId::ROOT, || {
        palette_groups(&sample(), &draw, &|_| EventHandler::new(|()| {}))
    });
    let titles: Vec<(&str, Option<&str>)> = groups
        .0
        .iter()
        .map(|g| (g.title.as_str(), g.action.as_ref().map(|(l, _)| l.as_str())))
        .collect();
    assert_eq!(
        titles,
        [
            ("Top Hit", None),
            ("Documents", Some("Show More")),
            ("Clipboard", None),
            ("Emoji", Some("Show Less")),
        ]
    );
    let row = |group: usize| match &groups.0[group].entries {
        GroupEntries::List(entries) => entries[0].clone(),
        GroupEntries::Grid(_) => panic!("a list"),
    };
    match row(0) {
        MenuEntry::Row(row) => {
            assert_eq!(
                row.trail,
                Trail::Note("Document".to_owned()),
                "Top Hit mixes kinds"
            );
            assert_eq!(
                row.shape,
                RowShape::File {
                    thumb: None,
                    location: "~/Documents".to_owned(),
                    modified: "Yesterday".to_owned(),
                }
            );
        }
        other => panic!("a file row: {other:?}"),
    }
    match row(1) {
        MenuEntry::Row(row) => assert_eq!(row.trail, Trail::None, "the date is in the shape"),
        other => panic!("a file row: {other:?}"),
    }
    match row(2) {
        MenuEntry::Row(row) => assert_eq!(
            row.shape,
            RowShape::Clip {
                body: ClipBody::Text {
                    excerpt: "hello".to_owned(),
                    lines: 2
                },
                age: "2m".to_owned(),
            }
        ),
        other => panic!("a clipboard row: {other:?}"),
    }
    match &groups.0[3].entries {
        GroupEntries::Grid(grid) => {
            assert_eq!(grid.cells[0].glyph, "🎉");
            assert_eq!(grid.cells[0].name, "party popper");
            assert_eq!(grid.columns, ds::EMOJI_COLUMNS);
        }
        GroupEntries::List(_) => panic!("the emoji grid"),
    }
}

/// F1040: every list row carries its first action's chord as `RowChord::on_selected`, so quire
/// shows it on the selected row only and moves it with the selection; the trail keeps only data.
#[test]
fn each_row_carries_its_chord_for_the_selection() {
    let reveal = RowChord::on_selected(shortcut(&chords::reveal()));
    let draw = Draw {
        tile_of: &tile,
        clip_image: &|_| None,
        now: NOW,
    };
    let mut dom = VirtualDom::new(|| rsx! {});
    dom.rebuild_in_place();
    let groups = dom.in_scope(ScopeId::ROOT, || {
        palette_groups(&sample(), &draw, &|_| EventHandler::new(|()| {}))
    });
    let rows: Vec<(RowChord, Trail)> = groups.0[..3]
        .iter()
        .map(|group| match &group.entries {
            GroupEntries::List(entries) => match &entries[0] {
                MenuEntry::Row(row) => (row.chord.clone(), row.trail.clone()),
                other => panic!("a shaped row: {other:?}"),
            },
            GroupEntries::Grid(_) => panic!("a list"),
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (reveal.clone(), Trail::Note("Document".to_owned())),
            (reveal.clone(), Trail::None),
            (reveal, Trail::None),
        ]
    );
    assert!(
        rows.iter()
            .all(|(chord, _)| chord.shown == ChordShown::Selected)
    );
}

#[test]
fn plain_sections_are_lists_with_the_m4_trails() {
    let got = palette_sections(&sample(), &tile);
    assert_eq!(got.len(), 4);
    let trails: Vec<Trail> = got
        .iter()
        .flat_map(|(_, entries)| entries.iter())
        .filter_map(|entry| match entry {
            MenuEntry::Row(row) => Some(row.trail.clone()),
            _ => None,
        })
        .collect();
    assert!(
        matches!(&trails[0], Trail::Note(_)),
        "a date, not a chord: {:?}",
        trails[0]
    );
    assert!(
        matches!(&trails[1], Trail::Note(_)),
        "a date: {:?}",
        trails[1]
    );
}
