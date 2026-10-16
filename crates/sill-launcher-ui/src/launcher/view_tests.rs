use sill_launcher::{
    Generation, IconRef, Item, ItemId, More, ProviderKind, Score, Section, SectionSpan,
    ShownSection, Subject,
};
use sill_model::results::Results;

use super::{Expanded, Layout, Line, View, toggled, view};
use crate::launcher::keys::Filter;
use crate::launcher::sections_view::HeaderAction;

fn item(provider: ProviderKind, title: &str) -> Item {
    Item {
        id: ItemId(title.to_owned()),
        provider,
        title: title.to_owned(),
        subtitle: String::new(),
        icon: IconRef::None,
        score: Score(1),
        actions: Vec::new(),
        subject: Subject::None,
    }
}

/// Top Hit, then eight documents (Documents caps at five), then one clipboard row.
fn results() -> Results {
    let mut items = vec![item(ProviderKind::Apps, "Files")];
    items.extend((0..8).map(|n| item(ProviderKind::Files, &format!("d{n}"))));
    items.push(item(ProviderKind::Clipboard, "secret text"));
    Results {
        generation: Generation(2),
        items,
        sections: vec![
            SectionSpan {
                section: Section::TopHit,
                rows: 1,
            },
            SectionSpan {
                section: Section::Documents,
                rows: 8,
            },
            SectionSpan {
                section: Section::Clipboard,
                rows: 1,
            },
        ],
    }
}

#[test]
fn the_runners_sections_show_capped_with_a_show_more_line() {
    let shown = view(
        &results(),
        Generation(1),
        "d",
        Filter::All,
        &Expanded::default(),
    );
    assert_eq!(shown.sections[1].more, More::Hidden(3));
    let lines = shown.lines();
    assert_eq!(
        lines.len(),
        1 + 5 + 1 + 1,
        "top hit, five docs, Show More, clip"
    );
    assert_eq!(
        lines[6],
        Line::More {
            section: Section::Documents,
            action: HeaderAction::ShowMore
        }
    );
    assert_eq!(shown.row(6), None, "a Show More line is not a row");
    assert_eq!(shown.row(1).map(|i| i.title), Some("d0".to_owned()));
    let report = shown.report("d", 6);
    assert_eq!(report.sections[1].hidden, 3);
    assert_eq!(
        report.sections[1].rows.last().map(String::as_str),
        Some("Show More")
    );
    assert_eq!(
        report.sections[2].rows,
        ["<clipboard>"],
        "no clipboard text"
    );
    let open = view(
        &results(),
        Generation(1),
        "d",
        Filter::All,
        &Expanded(vec![Section::Documents]),
    );
    assert_eq!(open.sections[1].rows.len(), 8);
    assert_eq!(
        open.report("d", 0).sections[1]
            .rows
            .last()
            .map(String::as_str),
        Some("Show Less")
    );
}

#[test]
fn the_line_accessors_agree_with_the_lines_they_stand_for() {
    for expanded in [Expanded::default(), Expanded(vec![Section::Documents])] {
        let shown = view(&results(), Generation(1), "d", Filter::All, &expanded);
        let lines = shown.lines();
        assert_eq!(shown.len(), lines.len());
        for (index, line) in lines.iter().enumerate() {
            match line {
                Line::Row(item) => {
                    assert_eq!(shown.row(index).as_ref(), Some(item), "row {index}");
                    assert_eq!(
                        shown.line_of(|found| found.id == item.id),
                        Some(index),
                        "line_of {index}"
                    );
                }
                Line::More { section, .. } => {
                    assert_eq!(shown.row(index), None, "row {index}");
                    assert_eq!(shown.more_line(*section), Some(index), "more_line {index}");
                }
            }
        }
        assert_eq!(shown.row(lines.len()), None, "past the last line");
        assert_eq!(
            shown.layout(lines.len()),
            Layout::List,
            "past the last line"
        );
    }
}

#[test]
fn a_filter_shows_its_provider_alone_and_stale_results_nothing() {
    let only = view(
        &results(),
        Generation(1),
        "d",
        Filter::Only(ProviderKind::Files),
        &Expanded::default(),
    );
    assert_eq!(only.sections.len(), 1);
    assert_eq!(only.sections[0].section, Section::Documents);
    assert_eq!(only.sections[0].rows.len(), 8, "a filter is never capped");
    let stale = view(
        &results(),
        Generation(2),
        "d",
        Filter::All,
        &Expanded::default(),
    );
    assert!(stale.is_empty());
}

#[test]
fn results_without_spans_are_placed_here() {
    let bare = Results {
        sections: Vec::new(),
        ..results()
    };
    let shown = view(&bare, Generation(1), "d", Filter::All, &Expanded::default());
    assert_eq!(shown.sections[0].section, Section::TopHit);
    assert_eq!(
        shown.providers(),
        [
            ProviderKind::Apps,
            ProviderKind::Files,
            ProviderKind::Clipboard
        ]
    );
}

#[test]
fn show_more_toggles_its_section() {
    let open = toggled(&Expanded::default(), Section::Documents);
    assert_eq!(open, Expanded(vec![Section::Documents]));
    assert_eq!(toggled(&open, Section::Documents), Expanded::default());
}

/// Where a section's "Show More" / "Show Less" line is, which the selection follows on a toggle.
#[test]
fn a_sections_more_line_moves_with_its_rows() {
    let opened = Expanded(vec![Section::Documents]);
    let cases = [
        (Expanded::default(), Section::Documents, Some(6)),
        (opened.clone(), Section::Documents, Some(9)),
        (opened, Section::Clipboard, None),
        (Expanded::default(), Section::TopHit, None),
    ];
    for (expanded, section, want) in cases {
        let shown = view(&results(), Generation(1), "d", Filter::All, &expanded);
        assert_eq!(shown.more_line(section), want, "{expanded:?} {section:?}");
    }
}

/// ← and → belong to the grid only on an Emoji section's cells: an emoji as the Top Hit
/// is a list row, and the Emoji header's "Show More" a list stop.
#[test]
fn only_an_emoji_sections_cells_are_grid_lines() {
    let emoji = |t: &str| item(ProviderKind::Emoji, t);
    let shown = View {
        sections: vec![
            ShownSection {
                section: Section::TopHit,
                rows: vec![emoji("🏮")],
                more: More::Nothing,
            },
            ShownSection {
                section: Section::Emoji,
                rows: vec![emoji("🎃"), emoji("🎉")],
                more: More::Hidden(4),
            },
            ShownSection {
                section: Section::Documents,
                rows: vec![item(ProviderKind::Files, "d0")],
                more: More::Nothing,
            },
        ],
    };
    let cases = [
        (0, Layout::List),
        (1, Layout::Grid),
        (2, Layout::Grid),
        (3, Layout::List),
        (4, Layout::List),
        (5, Layout::List),
    ];
    for (line, want) in cases {
        assert_eq!(shown.layout(line), want, "line {line}");
    }
}
