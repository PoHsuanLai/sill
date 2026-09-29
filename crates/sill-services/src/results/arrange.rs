//! The published order of the rows: Spotlight's sections (design/13 §13.3.9), placed by
//! `sill_launcher::place` and published with their spans, so the launcher reads the headings from
//! `Results::sections` instead of re-deriving them.
//!
//! - typed text, every provider: Top Hit, then one section per provider in the plan's order.
//! - nothing typed, every provider: Actions (the System rows), then Recent.
//! - a filtered query (`Scope::Only`, the Tab filter): that provider's rows alone under its own
//!   section, typed or not, uncapped by the plan's ceiling (the provider bounds its own
//!   answer; the emoji grid lists its recent-first set with nothing typed).
//!
//! Pure.

use sill_launcher::{
    Frecency, Item, Needle, Placement, ProviderKind, QueryShape, Scope, Section, SectionPlan,
    SectionSpan, UnixTime, place,
};

/// Whether anything was typed. Whitespace alone is nothing (the same rule as
/// [`Needle::is_empty`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Typed {
    Nothing,
    Text,
}

impl Typed {
    /// What `text` counts as.
    pub(crate) fn of(text: &str) -> Typed {
        if Needle::parse(text).is_empty() {
            Typed::Nothing
        } else {
            Typed::Text
        }
    }

    fn shape(self) -> QueryShape {
        match self {
            Typed::Nothing => QueryShape::Empty,
            Typed::Text => QueryShape::Text,
        }
    }
}

/// What the runner asked: the text's shape and the scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Asked {
    pub(crate) typed: Typed,
    pub(crate) scope: Scope,
}

impl Asked {
    /// Nothing asked yet, or the launcher closed.
    pub(crate) const IDLE: Asked = Asked {
        typed: Typed::Nothing,
        scope: Scope::All,
    };
}

/// `merged` (best first) as published under `asked`.
pub(crate) fn arranged(
    merged: &[Item],
    plan: &SectionPlan,
    asked: Asked,
    frecency: &Frecency,
    now: UnixTime,
) -> Placement {
    match asked.scope {
        Scope::All => place(merged, plan, asked.typed.shape(), frecency, now),
        Scope::Only(kind) => alone(merged, plan, kind),
    }
}

/// `kind`'s rows of `merged` under its own section; nothing when the plan has no rule for it.
fn alone(merged: &[Item], plan: &SectionPlan, kind: ProviderKind) -> Placement {
    let Some(section) = plan.rule(kind).map(|rule| rule.section) else {
        return Placement::default();
    };
    let items: Vec<Item> = merged
        .iter()
        .filter(|item| item.provider == kind)
        .cloned()
        .collect();
    Placement {
        spans: span(section, items.len()),
        items,
    }
}

fn span(section: Section, rows: usize) -> Vec<SectionSpan> {
    match u32::try_from(rows) {
        Ok(0) => Vec::new(),
        Ok(rows) => vec![SectionSpan { section, rows }],
        Err(_) => vec![SectionSpan {
            section,
            rows: u32::MAX,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::{Asked, Typed, arranged};
    use sill_launcher::{
        Frecency, IconRef, Item, ItemId, ItemKey, Placement, ProviderKind, Scope, Score, Section,
        SectionPlan, UnixTime,
    };

    use ProviderKind::{Apps, Emoji, Files, System};

    const NOW: UnixTime = UnixTime(1_790_000_000);

    fn item(provider: ProviderKind, title: &str, score: u32) -> Item {
        Item {
            id: ItemId(title.to_owned()),
            provider,
            title: title.to_owned(),
            subtitle: String::new(),
            icon: IconRef::None,
            score: Score(score),
            actions: Vec::new(),
            subject: sill_launcher::Subject::None,
        }
    }

    fn outline(placement: &Placement) -> Vec<(Section, Vec<&str>)> {
        let mut rows = placement.items.iter();
        placement
            .spans
            .iter()
            .map(|span| {
                let titles = rows
                    .by_ref()
                    .take(span.rows as usize)
                    .map(|i| i.title.as_str())
                    .collect();
                (span.section, titles)
            })
            .collect()
    }

    #[test]
    fn typing_is_classified_like_the_needle() {
        const CASES: &[(&str, Typed)] = &[
            ("", Typed::Nothing),
            ("   ", Typed::Nothing),
            ("f", Typed::Text),
            (" fi ", Typed::Text),
            ("2+2", Typed::Text),
        ];
        for &(text, want) in CASES {
            assert_eq!(Typed::of(text), want, "{text:?}");
        }
    }

    #[test]
    fn every_ask_places_its_rows() {
        let merged = vec![
            item(Apps, "Firefox", 900),
            item(Files, "firefox.pdf", 800),
            item(Emoji, "🦊", 700),
            item(System, "Lock", 100),
        ];
        let frecency =
            Frecency::default().record(ItemKey::of(Apps, &ItemId("Firefox".to_owned())), NOW);
        type Case<'a> = (&'a str, Asked, Vec<(Section, Vec<&'a str>)>);
        let cases: Vec<Case> = vec![
            (
                "typed text: Top Hit, then sections",
                Asked {
                    typed: Typed::Text,
                    scope: Scope::All,
                },
                vec![
                    (Section::TopHit, vec!["Firefox"]),
                    (Section::Documents, vec!["firefox.pdf"]),
                    (Section::Emoji, vec!["🦊"]),
                    (Section::System, vec!["Lock"]),
                ],
            ),
            (
                "nothing typed: Actions, then Recent",
                Asked::IDLE,
                vec![
                    (Section::Actions, vec!["Lock"]),
                    (Section::Recent, vec!["Firefox"]),
                ],
            ),
            (
                "filtered with nothing typed: the provider's rows alone",
                Asked {
                    typed: Typed::Nothing,
                    scope: Scope::Only(Emoji),
                },
                vec![(Section::Emoji, vec!["🦊"])],
            ),
            (
                "filtered with text: no Top Hit",
                Asked {
                    typed: Typed::Text,
                    scope: Scope::Only(Files),
                },
                vec![(Section::Documents, vec!["firefox.pdf"])],
            ),
            (
                "filtered to a provider with no rows: nothing",
                Asked {
                    typed: Typed::Text,
                    scope: Scope::Only(ProviderKind::Web),
                },
                vec![],
            ),
        ];
        let plan = SectionPlan::default_plan();
        for (name, asked, want) in cases {
            let placement = arranged(&merged, &plan, asked, &frecency, NOW);
            assert_eq!(outline(&placement), want, "{name}");
        }
    }

    #[test]
    fn a_filtered_section_is_not_cut_at_the_ceiling() {
        let merged: Vec<Item> = (0..30).map(|n| item(Emoji, &format!("e{n}"), 1)).collect();
        let asked = Asked {
            typed: Typed::Nothing,
            scope: Scope::Only(Emoji),
        };
        let placement = arranged(
            &merged,
            &SectionPlan::default_plan(),
            asked,
            &Frecency::default(),
            NOW,
        );
        assert_eq!(placement.items.len(), 30);
        assert_eq!(placement.spans[0].rows, 30);
    }
}
