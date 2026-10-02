use ds::components::lists::emoji_grid::grid::EMOJI_COLUMNS;
use std::path::PathBuf;

use dioxus::prelude::*;
use ds::components::lists::row::chord::ChordShown;
use ds::components::lists::row::shape::{ClipBody, RowShape};
use ds::components::menus::palette::palette_group::GroupEntries;
use ds::prelude::*;
use sill_launcher::{
    Action, ActionId, ActionKeys, CharCount, ClipFacts, ClipKey, ClipKind, EmojiFacts, Excerpt,
    FileFacts, IconRef, Item, ItemId, Mime, More, ProviderKind, Score, Section, ShownSection,
    SkinTone, Subject, UnixTime, chords,
};

use super::{Draw, HeaderAction, palette_groups};
use crate::launcher::chord_keys::shortcut;
use crate::launcher::entries::tile;

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
    let top = row(0);
    assert_eq!(
        top.accessory,
        Accessory::Text("Document".to_owned()),
        "Top Hit mixes kinds"
    );
    assert_eq!(
        top.shape,
        RowShape::File {
            thumb: None,
            location: "~/Documents".to_owned(),
            modified: "Yesterday".to_owned(),
        }
    );
    assert_eq!(
        row(1).accessory,
        Accessory::None,
        "the date is in the shape"
    );
    assert_eq!(
        row(2).shape,
        RowShape::Clip {
            body: ClipBody::Text {
                excerpt: "hello".to_owned(),
                lines: 2
            },
            age: "2m".to_owned(),
        }
    );
    match &groups.0[3].entries {
        GroupEntries::Grid(grid) => {
            assert_eq!(grid.cells[0].glyph, "🎉");
            assert_eq!(grid.cells[0].name, "party popper");
            assert_eq!(grid.columns, EMOJI_COLUMNS);
        }
        GroupEntries::List(_) => panic!("the emoji grid"),
    }
}

/// Every list row carries its first action's chord as `RowChord::on_selected`, so quire
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
    let rows: Vec<(RowChord, Accessory)> = groups.0[..3]
        .iter()
        .map(|group| match &group.entries {
            GroupEntries::List(entries) => (entries[0].chord.clone(), entries[0].accessory.clone()),
            GroupEntries::Grid(_) => panic!("a list"),
        })
        .collect();
    assert_eq!(
        rows,
        vec![
            (reveal.clone(), Accessory::Text("Document".to_owned())),
            (reveal.clone(), Accessory::None),
            (reveal, Accessory::None),
        ]
    );
    assert!(
        rows.iter()
            .all(|(chord, _)| chord.shown == ChordShown::Selected)
    );
}

/// An emoji row outside the grid (under Recent, or anywhere a mixed list shows one) is its name led
/// by the glyph in the icon slot. Its title is the glyph itself, which drew tiny above its own name.
#[test]
fn an_emoji_in_a_list_is_its_name_led_by_the_glyph() {
    let emoji = item(
        ProviderKind::Emoji,
        "📫",
        Subject::Emoji(EmojiFacts {
            glyph: "📫".to_owned(),
            name: "closed mailbox with raised flag".to_owned(),
            tone: SkinTone::Default,
        }),
    );
    let draw = Draw {
        tile_of: &tile,
        clip_image: &|_| None,
        now: NOW,
    };
    let section = ShownSection {
        section: Section::Recent,
        rows: vec![emoji],
        more: More::Nothing,
    };
    let mut dom = VirtualDom::new(|| rsx! {});
    dom.rebuild_in_place();
    let groups = dom.in_scope(ScopeId::ROOT, || {
        palette_groups(&[section], &draw, &|_| EventHandler::new(|()| {}))
    });
    let GroupEntries::List(entries) = &groups.0[0].entries else {
        panic!("a list");
    };
    let row = &entries[0];
    assert_eq!(row.title, "closed mailbox with raised flag".into());
    assert_eq!(row.detail, None);
    assert_eq!(row.leading, RowLeading::Text("📫".to_owned()));
}
