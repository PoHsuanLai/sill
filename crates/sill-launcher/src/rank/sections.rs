//! Spotlight's sections (M9, FINDINGS "M9 freeze" F645): the merged rows placed under Top Hit,
//! then one section per kind of thing (Applications, Documents, ... Web last), each provider
//! with a weight (how strongly its best row competes for Top Hit) and a cap (rows shown before
//! "Show More"). The runner publishes every placed row up to [`SectionPlan::ceiling`] with the
//! section spans ([`place`]); the launcher shows each section capped unless the person expanded
//! it ([`shown`]). Pure; the numbers are [`SectionPlan::DEFAULT`], proposed, with no settings key
//! yet (a key is a design/22 row first).

use serde::{Deserialize, Serialize};

use crate::frecency::Frecency;
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
    /// settings and system three, clipboard three, emoji a grid row of eight, and the web
    /// fallback one row that never takes Top Hit.
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
                rule(K::Clipboard, S::Clipboard, 600, 3),
                rule(K::Emoji, S::Emoji, 500, 8),
                rule(K::System, S::System, 800, 3),
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
/// Rules (lane d fills; `sections_tests` pins them): typed text: Top Hit is the row with the
/// highest [`weighted`] score among providers of non-zero weight (ties: the earlier row), and it
/// is not repeated below; then each section of `plan.order`, its rows best first, at most
/// `plan.ceiling`; empty sections are left out. Nothing typed: `Actions` (every System row) then
/// `Recent` (the `plan.recent` most frecent other rows at `now`, never-chosen rows left out;
/// Clipboard and Web rows are never Recent).
pub fn place(
    merged: &[Item],
    plan: &SectionPlan,
    shape: QueryShape,
    frecency: &Frecency,
    now: UnixTime,
) -> Placement {
    let _ = (merged, plan, shape, frecency, now);
    todo!("M9 fill: d-preview")
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
    let _ = (placement, plan, expanded);
    todo!("M9 fill: d-preview")
}

#[cfg(test)]
#[path = "sections_tests.rs"]
mod sections_tests;
