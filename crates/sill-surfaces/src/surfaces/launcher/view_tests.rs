use sill_launcher::{
    Generation, IconRef, Item, ItemId, More, ProviderKind, Score, Section, SectionSpan, Subject,
};
use sill_services::Results;

use super::{Expanded, Line, MORE_ID, more_ref, more_section, toggled, view};
use crate::surfaces::launcher::keys::Filter;
use crate::surfaces::launcher::sections_view::HeaderAction;

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
fn a_show_more_reference_names_its_section_and_toggles_it() {
    let reference = more_ref(Section::Documents).expect("documents have a provider");
    assert_eq!(reference.id.0, MORE_ID);
    assert_eq!(more_section(&reference), Some(Section::Documents));
    assert_eq!(more_ref(Section::TopHit), None);
    let open = toggled(&Expanded::default(), Section::Documents);
    assert_eq!(open, Expanded(vec![Section::Documents]));
    assert_eq!(toggled(&open, Section::Documents), Expanded::default());
}
