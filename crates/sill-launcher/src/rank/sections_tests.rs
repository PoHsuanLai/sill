use super::{
    Cap, Expanded, More, Placement, QueryShape, Section, SectionPlan, SectionSpan, Weight, place,
    shown, weighted,
};
use crate::frecency::{Frecency, ItemKey};
use crate::ids::{ItemId, ProviderKind};
use crate::item::{IconRef, Item};
use crate::rank::Score;
use crate::subject::Subject;
use crate::time::UnixTime;

use ProviderKind::{Apps, Calculator, Clipboard, Emoji, Files, System, Web, Windows};

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
        subject: Subject::None,
    }
}

/// Rows sorted as the runner keeps them (score, then provider, then title).
fn merged(rows: &[(ProviderKind, &str, u32)]) -> Vec<Item> {
    let mut items: Vec<Item> = rows.iter().map(|&(p, t, s)| item(p, t, s)).collect();
    items.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(a.provider.cmp(&b.provider))
            .then_with(|| a.title.cmp(&b.title))
    });
    items
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
fn the_default_plan_has_one_rule_per_provider_and_web_last() {
    let plan = SectionPlan::default_plan();
    for kind in ProviderKind::ALL {
        assert!(plan.rule(kind).is_some(), "{kind:?} has no rule");
    }
    assert_eq!(plan.order.last(), Some(&Section::Web));
    assert_eq!(plan.rule(Web).map(|r| r.weight), Some(Weight(0)));
}

#[test]
fn a_weight_scales_a_score_in_thousandths() {
    const CASES: &[(u32, u16, u32)] = &[
        (1000, 1000, 1000),
        (1000, 500, 500),
        (900, 850, 765),
        (1100, 0, 0),
    ];
    for &(score, weight, want) in CASES {
        assert_eq!(weighted(Score(score), Weight(weight)), Score(want));
    }
}

#[test]
fn typed_text_is_the_weighted_top_hit_then_sections_in_order() {
    let plan = SectionPlan::default_plan();
    type Case<'a> = (
        &'a str,
        Vec<(ProviderKind, &'a str, u32)>,
        Vec<(Section, Vec<&'a str>)>,
    );
    let cases: Vec<Case> = vec![
        ("nothing matched", vec![], vec![]),
        (
            "a document outranks an app by score but not by weight",
            vec![(Files, "firefox.pdf", 900), (Apps, "Firefox", 800)],
            vec![
                (Section::TopHit, vec!["Firefox"]),
                (Section::Documents, vec!["firefox.pdf"]),
            ],
        ),
        (
            "the web fallback never takes Top Hit and comes last",
            vec![(Web, "Search the web", 2000), (Windows, "foot ~", 300)],
            vec![
                (Section::TopHit, vec!["foot ~"]),
                (Section::Web, vec!["Search the web"]),
            ],
        ),
        (
            "sections follow the plan's order, not score",
            vec![
                (Emoji, "😀", 700),
                (Clipboard, "hello", 650),
                (Apps, "Files", 990),
                (Files, "notes.md", 400),
                (Calculator, "= 4", 100),
            ],
            vec![
                (Section::TopHit, vec!["Files"]),
                (Section::Documents, vec!["notes.md"]),
                (Section::Calculator, vec!["= 4"]),
                (Section::Clipboard, vec!["hello"]),
                (Section::Emoji, vec!["😀"]),
            ],
        ),
        (
            "emoji and clipboard rows never take Top Hit, whatever their score",
            vec![
                (Emoji, "✉", 900_000_000),
                (Clipboard, "mail me", 5000),
                (Apps, "Mailo", 950),
            ],
            vec![
                (Section::TopHit, vec!["Mailo"]),
                (Section::Clipboard, vec!["mail me"]),
                (Section::Emoji, vec!["✉"]),
            ],
        ),
        (
            "a system row that matches best is the Top Hit (typing logout finds Log Out first)",
            vec![(System, "Log Out", 1350), (Files, "logout-notes.md", 900)],
            vec![
                (Section::TopHit, vec!["Log Out"]),
                (Section::Documents, vec!["logout-notes.md"]),
            ],
        ),
        (
            "an app that matches as well as a system row is the Top Hit",
            vec![(System, "Lock Screen", 1000), (Apps, "Lockdown", 1000)],
            vec![
                (Section::TopHit, vec!["Lockdown"]),
                (Section::System, vec!["Lock Screen"]),
            ],
        ),
        (
            "with no primary row there is no Top Hit at all",
            vec![(Emoji, "✉", 900_000_000), (Web, "Search the web", 2000)],
            vec![
                (Section::Emoji, vec!["✉"]),
                (Section::Web, vec!["Search the web"]),
            ],
        ),
    ];
    for (name, rows, want) in cases {
        let placement = place(
            &merged(&rows),
            &plan,
            QueryShape::Text,
            &Frecency::default(),
            NOW,
        );
        assert_eq!(outline(&placement), want, "{name}");
        let covered: u32 = placement.spans.iter().map(|s| s.rows).sum();
        assert_eq!(
            covered as usize,
            placement.items.len(),
            "{name}: spans cover every row"
        );
    }
}

#[test]
fn a_section_publishes_up_to_the_ceiling() {
    let plan = SectionPlan {
        ceiling: Cap(4),
        ..SectionPlan::default_plan()
    };
    let names: Vec<String> = (0..9).map(|n| format!("doc{n}")).collect();
    let rows: Vec<(ProviderKind, &str, u32)> = names
        .iter()
        .enumerate()
        .map(|(n, t)| (Files, t.as_str(), 500 - n as u32))
        .collect();
    let placement = place(
        &merged(&rows),
        &plan,
        QueryShape::Text,
        &Frecency::default(),
        NOW,
    );
    assert_eq!(
        outline(&placement),
        vec![
            (Section::TopHit, vec!["doc0"]),
            (Section::Documents, vec!["doc1", "doc2", "doc3", "doc4"]),
        ]
    );
}

#[test]
fn nothing_typed_is_actions_then_recent_without_clipboard_or_web() {
    let plan = SectionPlan::default_plan();
    let rows = merged(&[
        (System, "Lock", 0),
        (Apps, "Files", 0),
        (Clipboard, "secret-ish", 0),
        (Web, "Search the web", 0),
        (Emoji, "😀", 0),
    ]);
    let key = |p, id: &str| ItemKey::of(p, &ItemId(id.to_owned()));
    let frecency = Frecency::default()
        .record(key(Apps, "Files"), NOW)
        .record(key(Clipboard, "secret-ish"), NOW)
        .record(key(Web, "Search the web"), NOW)
        .record(key(Emoji, "😀"), NOW)
        .record(key(Emoji, "😀"), NOW);
    let placement = place(&rows, &plan, QueryShape::Empty, &frecency, NOW);
    assert_eq!(
        outline(&placement),
        vec![
            (Section::Actions, vec!["Lock"]),
            (Section::Recent, vec!["😀", "Files"]),
        ]
    );
}

#[test]
fn a_section_shows_its_cap_until_expanded() {
    let plan = SectionPlan::default_plan();
    let docs: Vec<Item> = (0..8).map(|n| item(Files, &format!("d{n}"), 100)).collect();
    let placement = Placement {
        items: std::iter::once(item(Apps, "Files", 900))
            .chain(docs)
            .collect(),
        spans: vec![
            SectionSpan {
                section: Section::TopHit,
                rows: 1,
            },
            SectionSpan {
                section: Section::Documents,
                rows: 8,
            },
        ],
    };
    let capped = shown(&placement, &plan, &Expanded::default());
    assert_eq!(capped.len(), 2);
    assert_eq!(capped[0].more, More::Nothing);
    assert_eq!(capped[1].rows.len(), 5, "Documents' cap");
    assert_eq!(capped[1].more, More::Hidden(3));
    let open = shown(&placement, &plan, &Expanded(vec![Section::Documents]));
    assert_eq!(open[1].rows.len(), 8);
    assert_eq!(open[1].more, More::Expanded);
}
