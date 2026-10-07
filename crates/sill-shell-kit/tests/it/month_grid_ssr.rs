//! MonthGrid as markup (design/04-COMPONENTS.md section 39): the goldens are
//! `tests/snapshots/month_grid/<name>.html` (plain, week numbers, pressable), each linted and
//! every `ds-` class in it styled by the stylesheet; and the markup says what the data says.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test it month_grid_ssr` rewrites the goldens.

use crate::support::golden;
use crate::support::month_sample;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::prelude::*;
use ds_lint::{LintConfig, markup};
use month_sample::{First, SEPTEMBER, sample};
use sill_shell_kit::month_grid::data::DayMark;
use sill_shell_kit::month_grid::data::DayPlace;
use sill_shell_kit::month_grid::data::IsoWeek;
use sill_shell_kit::month_grid::data::MonthKey;
use sill_shell_kit::month_grid::data::{DayKey, MonthStep, WeekNumbers};
use sill_shell_kit::prelude::*;

/// One specimen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case {
    /// Monday first, no week numbers, no buttons, days as text; light.
    Plain,
    /// Sunday first, week numbers, the step buttons; dark.
    WeekNumbers,
    /// Monday first, the step buttons, pressable days; light.
    Pressable,
}

const CASES: [(Case, &str); 3] = [
    (Case::Plain, "plain"),
    (Case::WeekNumbers, "week-numbers"),
    (Case::Pressable, "pressable"),
];

#[derive(Props, Clone, PartialEq)]
struct CaseProps {
    case: Case,
}

#[allow(non_snake_case)]
fn Specimen(props: CaseProps) -> Element {
    let (theme, first, weeks) = match props.case {
        Case::Plain | Case::Pressable => (Theme::Light, First::Monday, WeekNumbers::Hide),
        Case::WeekNumbers => (Theme::Dark, First::Sunday, WeekNumbers::Show),
    };
    let onstep = match props.case {
        Case::Plain => None,
        Case::WeekNumbers | Case::Pressable => Some(EventHandler::new(|_: MonthStep| {})),
    };
    let onpick = match props.case {
        Case::Plain | Case::WeekNumbers => None,
        Case::Pressable => Some(EventHandler::new(|_: DayKey| {})),
    };
    rsx! {
        Ds {
            appearance: Appearance { theme, ..Appearance::default() },
            material: Material::Popover,
            stylesheet: Inject::Host,
            MonthGrid { data: sample(SEPTEMBER, first), weeks, onstep, onpick }
        }
    }
}

fn render(case: Case) -> String {
    let mut dom = VirtualDom::new_with_props(Specimen, CaseProps { case });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn every_specimen_matches_its_golden() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|&(case, name)| {
            golden::check(&format!("month_grid/{name}.html"), &render(case)).err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_specimen_lints_clean_and_every_class_is_styled() {
    let sheet = sill_shell_kit::stylesheet();
    let mut failures = Vec::new();
    for (case, name) in CASES {
        let html = render(case);
        for offence in markup(&html, sheet, &LintConfig::new(&sill_shell_kit::kits())) {
            failures.push(format!("{name}: {:?} {}", offence.rule, offence.text));
        }
        for class in html
            .split("class=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .flat_map(str::split_whitespace)
            .filter(|class| class.starts_with("ds-"))
        {
            let needle = format!(".{class}");
            let styled = sheet.match_indices(&needle).any(|(at, _)| {
                !sheet[at + needle.len()..]
                    .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            });
            if !styled {
                failures.push(format!("{name}: .{class} is not styled"));
            }
        }
    }
    failures.dedup();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The sample is the month sill's own test pins: September 2026 from Monday is five rows from
/// 31 August (ISO week 36) to 4 October; from Sunday it starts on 30 August, still week 36.
#[test]
fn the_sample_is_the_month_sill_pins() {
    let monday = sample(SEPTEMBER, First::Monday);
    assert_eq!(monday.weeks.len(), 5);
    assert_eq!(monday.weeks[0].number, IsoWeek(36));
    assert_eq!(monday.weeks[0].days[0].key.day, 31);
    assert_eq!(monday.weeks[0].days[0].place, DayPlace::Before);
    assert_eq!(monday.weeks[3].days[5].mark, DayMark::Today);
    assert_eq!(monday.weeks[4].days[6].key.day, 4);
    assert_eq!(monday.weeks[4].days[6].place, DayPlace::After);
    let sunday = sample(SEPTEMBER, First::Sunday);
    assert_eq!(sunday.weeks[0].days[0].key.day, 30);
    assert_eq!(sunday.weeks[0].number, IsoWeek(36));
    let feb = sample(
        MonthKey {
            year: 2027,
            month: 2,
        },
        First::Monday,
    );
    assert_eq!(
        feb.weeks.len(),
        4,
        "a February that fits four weeks has four"
    );
}

/// Each form writes what its data says: one head per column (and an empty week cell when
/// numbers show), the rows' week numbers only when asked, today's `aria-current`, a dot per
/// busy day, the neighbours' places, the step buttons only with `onstep` and buttons for days
/// only with `onpick`; and nothing slides on the first render.
#[test]
fn the_markup_says_what_the_data_says() {
    let plain = render(Case::Plain);
    assert_eq!(plain.matches("class=\"ds-month-head\"").count(), 7);
    assert_eq!(plain.matches("class=\"ds-month-week\"").count(), 0);
    assert_eq!(plain.matches("class=\"ds-month-day\"").count(), 35);
    assert_eq!(plain.matches("aria-current=\"date\"").count(), 1);
    assert_eq!(plain.matches("class=\"ds-month-dot\"").count(), 4);
    assert_eq!(plain.matches("data-place=\"before\"").count(), 1);
    assert_eq!(plain.matches("data-place=\"after\"").count(), 4);
    assert!(!plain.contains("<button"), "no buttons without handlers");
    assert!(plain.contains("data-weeks=\"hide\""));

    let numbered = render(Case::WeekNumbers);
    assert_eq!(numbered.matches("class=\"ds-month-week\"").count(), 1 + 5);
    for week in 36..=40 {
        assert!(
            numbered.contains(&format!("class=\"ds-month-week\">{week}</span>")),
            "week {week}"
        );
    }
    assert!(numbered.contains("aria-label=\"Previous month\""));
    assert!(numbered.contains("aria-label=\"Next month\""));
    assert!(!numbered.contains("data-kind=\"pressable\""));

    let pressable = render(Case::Pressable);
    assert_eq!(pressable.matches("data-kind=\"pressable\"").count(), 35);
    for html in [&plain, &numbered, &pressable] {
        assert!(!html.contains("a-slide"), "the first month drawn is still");
    }
}
