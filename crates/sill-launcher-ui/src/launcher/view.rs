//! What the panel shows over the published results: the runner's sections
//! (`Results::sections`, placed by `sill_launcher::place`) capped by [`shown`] unless expanded,
//! or, under a Tab filter, that provider's rows alone under its own heading. The palette's
//! cursor stops on each row (each emoji cell) and then on the section header's "Show More" or
//! "Show Less" (quire's `PaletteGroup` action), so a line here is a row or that action,
//! numbered as the palette's `selected` counts them. Pure.

use sill_launcher::{
    Frecency, Generation, Item, More, Placement, ProviderKind, QueryShape, Section, SectionPlan,
    ShownSection, UnixTime, place, shown,
};
use sill_model::results::Results;

use crate::launcher::groups::fresh;
use crate::launcher::keys::Filter;
use crate::launcher::sections_view::HeaderAction;
use sill_ui_kit::links::{CLIPBOARD_TITLE, ShownRows, ShownRowsSection};

pub use sill_launcher::Expanded;

/// One line of the palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    Row(Item),
    /// A section header's trailing action, a stop after the section's rows.
    More {
        section: Section,
        action: HeaderAction,
    },
}

/// How the palette lays out a line: a list row, or a cell of the Emoji section's grid (where ←
/// and → move the cursor, design/13 §13.3.9 "2D arrow keys").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layout {
    List,
    Grid,
}

/// The sections the panel shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct View {
    pub sections: Vec<ShownSection>,
}

/// What the panel shows of `results` for an opening over generation `since`, typed `query`,
/// under `filter`, with `expanded` sections opened.
pub fn view(
    results: &Results,
    since: Generation,
    query: &str,
    filter: Filter,
    expanded: &Expanded,
) -> View {
    let plan = SectionPlan::default_plan();
    let items = fresh(results, since);
    let sections = match filter {
        Filter::Only(kind) => alone(items, &plan, kind),
        Filter::All => shown(&placed(items, &results.sections, query), &plan, expanded),
    };
    View { sections }
}

/// The runner's placement, or (a runner that published no spans that cover its rows: a test's
/// fake) the rows placed here with no frecency.
fn placed(items: &[Item], spans: &[sill_launcher::SectionSpan], query: &str) -> Placement {
    let covered: u64 = spans.iter().map(|span| u64::from(span.rows)).sum();
    if items.is_empty() {
        return Placement::default();
    }
    if !spans.is_empty() && covered == items.len() as u64 {
        return Placement {
            items: items.to_vec(),
            spans: spans.to_vec(),
        };
    }
    let shape = match query.trim() {
        "" => QueryShape::Empty,
        _ => QueryShape::Text,
    };
    let plan = SectionPlan::default_plan();
    place(items, &plan, shape, &Frecency::default(), UnixTime(0))
}

fn alone(items: &[Item], plan: &SectionPlan, kind: ProviderKind) -> Vec<ShownSection> {
    let rows: Vec<Item> = items
        .iter()
        .filter(|item| item.provider == kind)
        .cloned()
        .collect();
    match (plan.rule(kind), rows.is_empty()) {
        (Some(rule), false) => vec![ShownSection {
            section: rule.section,
            rows,
            more: More::Nothing,
        }],
        _ => Vec::new(),
    }
}

impl View {
    /// Every line, in the palette's order.
    pub fn lines(&self) -> Vec<Line> {
        self.sections.iter().flat_map(section_lines).collect()
    }

    /// The line of `section`'s "Show More" or "Show Less", when it has one.
    pub fn more_line(&self, section: Section) -> Option<usize> {
        self.lines()
            .iter()
            .position(|line| matches!(line, Line::More { section: at, .. } if *at == section))
    }

    /// The line of the first row `wanted` says yes to.
    pub fn line_of(&self, wanted: impl Fn(&Item) -> bool) -> Option<usize> {
        self.lines()
            .iter()
            .position(|line| matches!(line, Line::Row(item) if wanted(item)))
    }

    /// The row on line `index`, when that line is a row.
    pub fn row(&self, index: usize) -> Option<Item> {
        match self.lines().into_iter().nth(index)? {
            Line::Row(item) => Some(item),
            Line::More { .. } => None,
        }
    }

    /// How line `index` is laid out: a grid cell for an Emoji row (the palette draws that
    /// section as `PaletteGroup::grid`), else a list line (a header's action included).
    pub fn layout(&self, index: usize) -> Layout {
        let at = self
            .sections
            .iter()
            .flat_map(|s| {
                section_lines(s)
                    .into_iter()
                    .map(move |line| (s.section, line))
            })
            .nth(index);
        match at {
            Some((Section::Emoji, Line::Row(_))) => Layout::Grid,
            _ => Layout::List,
        }
    }

    /// How many lines there are.
    pub fn len(&self) -> usize {
        self.sections.iter().map(|s| section_lines(s).len()).sum()
    }

    /// Whether nothing shows.
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// Every row, with the "Show More" lines left out (what a pick is looked up among).
    pub fn rows(&self) -> Vec<&Item> {
        self.sections.iter().flat_map(|s| s.rows.iter()).collect()
    }

    /// The providers that show a row (Tab's ring under typed text).
    pub fn providers(&self) -> Vec<ProviderKind> {
        let mut kinds: Vec<ProviderKind> = self.rows().iter().map(|item| item.provider).collect();
        kinds.sort();
        kinds.dedup();
        kinds
    }

    /// The panel as `sill debug launcher-rows` reports it (clipboard titles withheld).
    pub fn report(&self, query: &str, selected: usize) -> ShownRows {
        ShownRows {
            query: query.to_owned(),
            sections: self
                .sections
                .iter()
                .map(|section| ShownRowsSection {
                    title: section.section.title().to_owned(),
                    rows: section_lines(section).iter().map(line_title).collect(),
                    hidden: match section.more {
                        More::Hidden(n) => n,
                        More::Nothing | More::Expanded => 0,
                    },
                })
                .collect(),
            selected: u32::try_from(selected).unwrap_or(u32::MAX),
            preview: None,
        }
    }
}

fn section_lines(section: &ShownSection) -> Vec<Line> {
    let rows = section.rows.iter().cloned().map(Line::Row);
    let more = match HeaderAction::of(section.more) {
        HeaderAction::None => None,
        action => Some(Line::More {
            section: section.section,
            action,
        }),
    };
    rows.chain(more).collect()
}

fn line_title(line: &Line) -> String {
    match line {
        Line::Row(item) if item.provider == ProviderKind::Clipboard => CLIPBOARD_TITLE.to_owned(),
        Line::Row(item) => item.title.clone(),
        Line::More { action, .. } => action.label().unwrap_or_default().to_owned(),
    }
}

/// `expanded` with `section` opened, or closed when it was open.
pub fn toggled(expanded: &Expanded, section: Section) -> Expanded {
    let mut sections: Vec<Section> = expanded
        .0
        .iter()
        .copied()
        .filter(|s| *s != section)
        .collect();
    if !expanded.0.contains(&section) {
        sections.push(section);
    }
    Expanded(sections)
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod view_tests;
