//! Spotlight's sections: the merged rows placed under Top Hit, then one section per kind of thing
//! (Applications, Documents, ... Web last), each provider with a weight (how strongly its best row
//! competes for Top Hit) and a cap (rows shown before "Show More"). The runner publishes every
//! placed row up to [`SectionPlan::ceiling`] with the section spans ([`place`]); the launcher shows
//! each section capped unless the person expanded it ([`shown`]). Pure; the numbers are
//! [`SectionPlan::DEFAULT`], proposed, with no settings key yet (a key is a design/22 row first).

use serde::{Deserialize, Serialize};

use crate::frecency::{Frecency, FrecencyScore, ItemKey};
use crate::ids::ProviderKind;
use crate::item::Item;
use crate::rank::Score;
use crate::time::UnixTime;

/// A heading in the results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Section {
    /// The one best row of all (typed text).
    TopHit,
    /// The system rows with nothing typed.
    Actions,
    /// The most frecent rows with nothing typed.
    Recent,
    Applications,
    Documents,
    Windows,
    Calculator,
    Settings,
    Clipboard,
    Emoji,
    System,
    /// Things apps index.
    Things,
    /// Computer-use runs.
    Runs,
    /// What the companion remembers.
    Memory,
    /// "Ask the companion …", just above the web fallback.
    Companion,
    /// "Search the web for …", always last.
    Web,
}

impl Section {
    /// The heading's words.
    pub fn title(self) -> &'static str {
        match self {
            Section::TopHit => "Top Hit",
            Section::Actions => "Actions",
            Section::Recent => "Recent",
            Section::Applications => "Applications",
            Section::Documents => "Documents",
            Section::Windows => "Windows",
            Section::Calculator => "Calculator",
            Section::Settings => "Settings",
            Section::Clipboard => "Clipboard",
            Section::Emoji => "Emoji",
            Section::System => "System",
            Section::Things => "Things",
            Section::Runs => "Runs",
            Section::Memory => "Memory",
            Section::Companion => "Assistant",
            Section::Web => "Web",
        }
    }
}

/// How strongly a provider's rows compete for Top Hit, in thousandths of their score: 1000 is
/// as ranked, 0 never takes Top Hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Weight(pub u16);

/// Rows a section shows before "Show More".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cap(pub u16);

/// One provider's place in the results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderRule {
    pub kind: ProviderKind,
    pub section: Section,
    pub weight: Weight,
    pub cap: Cap,
}

/// Every provider's rule, and the section order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SectionPlan {
    /// One rule per provider (a provider without one is placed nowhere).
    pub rules: Vec<ProviderRule>,
    /// The sections under Top Hit, in order.
    pub order: Vec<Section>,
    /// The most rows of one section the runner publishes (what "Show More" can reveal).
    pub ceiling: Cap,
    /// Rows under Recent with nothing typed.
    pub recent: Cap,
}

impl SectionPlan {
    /// The proposed numbers: apps rank as they are and show six (design/06 §11.3's mail cap),
    /// documents a little under them and five, windows three, the calculator its one answer,
    /// settings and system three, clipboard three, emoji a grid row of eight, things five, runs
    /// three, memory five, the companion's rows three and the web fallback one row. Top Hit
    /// comes from the primary providers only (apps, documents, windows, the calculator,
    /// settings), as Spotlight's does: the secondary ones (system commands, clipboard, emoji,
    /// things, runs, memory, companion, web) have weight 0. Emoji rows also score on a scale of
    /// their own (the position bits that order the grid), which no weight could compare with an
    /// app's.
    pub fn default_plan() -> SectionPlan {
        use ProviderKind as K;
        use Section as S;
        let rule = |kind, section, weight, cap| ProviderRule {
            kind,
            section,
            weight: Weight(weight),
            cap: Cap(cap),
        };
        SectionPlan {
            rules: vec![
                rule(K::Apps, S::Applications, 1000, 6),
                rule(K::Files, S::Documents, 850, 5),
                rule(K::Windows, S::Windows, 900, 3),
                rule(K::Calculator, S::Calculator, 1000, 1),
                rule(K::Settings, S::Settings, 700, 3),
                rule(K::Clipboard, S::Clipboard, 0, 3),
                rule(K::Emoji, S::Emoji, 0, 8),
                rule(K::System, S::System, 0, 3),
                rule(K::Things, S::Things, 0, 5),
                rule(K::Runs, S::Runs, 0, 3),
                rule(K::Memory, S::Memory, 0, 5),
                rule(K::Companion, S::Companion, 0, 3),
                rule(K::Web, S::Web, 0, 1),
            ],
            order: vec![
                S::Applications,
                S::Documents,
                S::Windows,
                S::Calculator,
                S::Settings,
                S::Clipboard,
                S::Emoji,
                S::System,
                S::Things,
                S::Runs,
                S::Memory,
                S::Companion,
                S::Web,
            ],
            ceiling: Cap(20),
            recent: Cap(5),
        }
    }

    /// `kind`'s rule.
    pub fn rule(&self, kind: ProviderKind) -> Option<&ProviderRule> {
        self.rules.iter().find(|rule| rule.kind == kind)
    }
}

impl Default for SectionPlan {
    fn default() -> Self {
        SectionPlan::default_plan()
    }
}

/// `score` scaled by `weight`: what a row brings to the Top Hit contest.
pub fn weighted(score: Score, weight: Weight) -> Score {
    Score(score.0.saturating_mul(u32::from(weight.0)) / 1000)
}

/// Whether anything was typed (whitespace alone is nothing, as `Needle::is_empty`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryShape {
    Empty,
    Text,
}

/// A run of consecutive rows under one heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SectionSpan {
    pub section: Section,
    pub rows: u32,
}

/// The rows as published: in section order, with their spans (which cover every row).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Placement {
    pub items: Vec<Item>,
    pub spans: Vec<SectionSpan>,
}

/// `merged` (best first, as the runner keeps it) placed in sections.
///
/// Rules (`sections_tests` pins them): typed text: Top Hit is the row with the highest [`weighted`]
/// score among providers of non-zero weight (ties: the earlier row), and it is not repeated below;
/// then each section of `plan.order`, its rows best first, at most `plan.ceiling`; empty sections
/// are left out. Nothing typed: `Actions` (every System row) then `Recent` (the `plan.recent` most
/// frecent other rows at `now`, never-chosen rows left out; Clipboard and Web rows are never
/// Recent).
pub fn place(
    merged: &[Item],
    plan: &SectionPlan,
    shape: QueryShape,
    frecency: &Frecency,
    now: UnixTime,
) -> Placement {
    let groups = match shape {
        QueryShape::Text => typed(merged, plan),
        QueryShape::Empty => untyped(merged, plan, frecency, now),
    };
    let groups: Vec<(Section, Vec<Item>)> = groups
        .into_iter()
        .filter(|(_, rows)| !rows.is_empty())
        .collect();
    Placement {
        spans: groups
            .iter()
            .map(|(section, rows)| SectionSpan {
                section: *section,
                rows: u32::try_from(rows.len()).unwrap_or(u32::MAX),
            })
            .collect(),
        items: groups.into_iter().flat_map(|(_, rows)| rows).collect(),
    }
}

/// The index of the Top Hit in `merged`: the highest weighted score of a non-zero weight, the
/// earlier row on a tie.
fn top_hit(merged: &[Item], plan: &SectionPlan) -> Option<usize> {
    merged
        .iter()
        .enumerate()
        .filter_map(|(at, item)| {
            let weight = plan.rule(item.provider)?.weight;
            (weight.0 > 0).then(|| (at, weighted(item.score, weight)))
        })
        .fold(
            None,
            |best: Option<(usize, Score)>, (at, score)| match best {
                Some((_, top)) if top >= score => best,
                _ => Some((at, score)),
            },
        )
        .map(|(at, _)| at)
}

fn typed(merged: &[Item], plan: &SectionPlan) -> Vec<(Section, Vec<Item>)> {
    let top = top_hit(merged, plan);
    let ceiling = usize::from(plan.ceiling.0);
    let head = top
        .map(|at| (Section::TopHit, vec![merged[at].clone()]))
        .into_iter();
    let body = plan.order.iter().map(|&section| {
        let rows = merged
            .iter()
            .enumerate()
            .filter(|&(at, item)| {
                Some(at) != top
                    && plan.rule(item.provider).map(|rule| rule.section) == Some(section)
            })
            .map(|(_, item)| item.clone())
            .take(ceiling)
            .collect();
        (section, rows)
    });
    head.chain(body).collect()
}

fn untyped(
    merged: &[Item],
    plan: &SectionPlan,
    frecency: &Frecency,
    now: UnixTime,
) -> Vec<(Section, Vec<Item>)> {
    let actions: Vec<Item> = merged
        .iter()
        .filter(|item| item.provider == ProviderKind::System)
        .cloned()
        .collect();
    let mut chosen: Vec<(FrecencyScore, usize)> = merged
        .iter()
        .enumerate()
        .filter(|(_, item)| never_recent(item.provider).is_none())
        .map(|(at, item)| {
            (
                frecency.score(&ItemKey::of(item.provider, &item.id), now),
                at,
            )
        })
        .filter(|(score, _)| *score > FrecencyScore::NONE)
        .collect();
    chosen.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let recent = chosen
        .into_iter()
        .take(usize::from(plan.recent.0))
        .map(|(_, at)| merged[at].clone())
        .collect();
    vec![(Section::Actions, actions), (Section::Recent, recent)]
}

/// Why a provider's rows are never Recent: System rows are Actions already, clipboard use is
/// never counted, and the web row says nothing about what was searched.
fn never_recent(kind: ProviderKind) -> Option<&'static str> {
    match kind {
        ProviderKind::System => Some("already under Actions"),
        ProviderKind::Clipboard => Some("clipboard use is not counted"),
        ProviderKind::Companion | ProviderKind::Runs | ProviderKind::Memory => {
            Some("an answer, a run and a memory row are not habits")
        }
        ProviderKind::Web => Some("the web row is the same row for every search"),
        _ => None,
    }
}

/// Sections the person opened with "Show More".
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Expanded(pub Vec<Section>);

/// Whether a section hides rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum More {
    /// Every row shows; no header action.
    Nothing,
    /// This many rows are hidden: the header offers "Show More".
    Hidden(u32),
    /// Expanded past its cap: the header offers "Show Less".
    Expanded,
}

/// One section as the launcher draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownSection {
    pub section: Section,
    pub rows: Vec<Item>,
    pub more: More,
}

/// `placement`'s sections as shown: each capped at its provider's cap unless `expanded` names
/// it (Top Hit, Actions and Recent are never capped further). Lane d fills; `sections_tests`.
pub fn shown(placement: &Placement, plan: &SectionPlan, expanded: &Expanded) -> Vec<ShownSection> {
    let mut rows = placement.items.iter();
    placement
        .spans
        .iter()
        .map(|span| {
            let all: Vec<Item> = rows
                .by_ref()
                .take(usize::try_from(span.rows).unwrap_or(usize::MAX))
                .cloned()
                .collect();
            capped(span.section, all, cap_of(span.section, plan), expanded)
        })
        .collect()
}

/// A section's cap: its provider's, none for Top Hit, Actions and Recent.
fn cap_of(section: Section, plan: &SectionPlan) -> Option<usize> {
    match section {
        Section::TopHit | Section::Actions | Section::Recent => None,
        _ => plan
            .rules
            .iter()
            .find(|rule| rule.section == section)
            .map(|rule| usize::from(rule.cap.0)),
    }
}

fn capped(
    section: Section,
    mut rows: Vec<Item>,
    cap: Option<usize>,
    expanded: &Expanded,
) -> ShownSection {
    let more = match cap {
        Some(cap) if rows.len() > cap => {
            if expanded.0.contains(&section) {
                More::Expanded
            } else {
                let hidden = rows.split_off(cap).len();
                More::Hidden(u32::try_from(hidden).unwrap_or(u32::MAX))
            }
        }
        _ => More::Nothing,
    };
    ShownSection {
        section,
        rows,
        more,
    }
}

#[cfg(test)]
#[path = "sections_tests.rs"]
mod sections_tests;
