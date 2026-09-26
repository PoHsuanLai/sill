use std::path::PathBuf;

use dioxus::prelude::*;
use ds::{ClipBody, GroupEntries, MenuEntry, RowShape, Shortcut, Trail};
use sill_launcher::{
    Action, ActionId, ActionKeys, CharCount, ClipFacts, ClipKey, ClipKind, EmojiFacts, Excerpt,
    FileFacts, IconRef, Item, ItemId, Mime, More, ProviderKind, Score, Section, ShownSection,
    SkinTone, Subject, UnixTime, chords,
};

use super::{Draw, HeaderAction, palette_groups, palette_sections};
use crate::surfaces::launcher::chord_keys::shortcut;
use crate::surfaces::launcher::entries::{Cursor, tile};

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
        cursor: Cursor(0),
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
            assert_eq!(row.trail, Trail::Shortcut(Shortcut(vec![ds::Key::Enter])));
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
        MenuEntry::Row(row) => assert_eq!(row.trail, Trail::Shortcut(shortcut(&chords::reveal()))),
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

/// F742: `↵` trails the row the palette's cursor is on, counted as the palette counts its stops
/// (each row, each emoji cell, a header's "Show More"); the other rows keep their chord. On a
/// "Show More" stop or an emoji cell no row trails `↵`.
#[test]
fn the_return_glyph_follows_the_cursor() {
    let enter = Trail::Shortcut(Shortcut(vec![ds::Key::Enter]));
    let reveal = Trail::Shortcut(shortcut(&chords::reveal()));
    // The sample's stops: a.pdf 0, b.pdf 1, Show More 2, hello 3, 🎉 4, Show Less 5.
    let cases: Vec<(usize, [&Trail; 3])> = vec![
        (0, [&enter, &reveal, &reveal]),
        (1, [&reveal, &enter, &reveal]),
        (2, [&reveal, &reveal, &reveal]),
        (3, [&reveal, &reveal, &enter]),
        (4, [&reveal, &reveal, &reveal]),
    ];
    let mut dom = VirtualDom::new(|| rsx! {});
    dom.rebuild_in_place();
    for (line, want) in cases {
        let draw = Draw {
            tile_of: &tile,
            clip_image: &|_| None,
            now: NOW,
            cursor: Cursor(line),
        };
        let groups = dom.in_scope(ScopeId::ROOT, || {
            palette_groups(&sample(), &draw, &|_| EventHandler::new(|()| {}))
        });
        let trails: Vec<Trail> = groups.0[..3]
            .iter()
            .map(|group| match &group.entries {
                GroupEntries::List(entries) => match &entries[0] {
                    MenuEntry::Row(row) => row.trail.clone(),
                    other => panic!("a shaped row: {other:?}"),
                },
                GroupEntries::Grid(_) => panic!("a list"),
            })
            .collect();
        let want: Vec<Trail> = want.into_iter().cloned().collect();
        assert_eq!(trails, want, "cursor on stop {line}");
    }
}

#[test]
fn plain_sections_are_lists_with_the_m4_trails() {
    let got = palette_sections(&sample(), &tile);
    assert_eq!(got.len(), 4);
    let trails: Vec<Trail> = got
        .iter()
        .flat_map(|(_, entries)| entries.iter())
        .filter_map(|entry| match entry {
            MenuEntry::Item { trail, .. } => Some(trail.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(trails[0], Trail::Shortcut(Shortcut(vec![ds::Key::Enter])));
    assert!(
        matches!(&trails[1], Trail::Note(_)),
        "a date: {:?}",
        trails[1]
    );
}
