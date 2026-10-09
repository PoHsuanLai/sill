//! MonthGrid's density as markup (design/04-COMPONENTS.md section 39): forced
//! compact, and `Auto` inside a small desktop `WidgetFrame` (compact) and a medium one
//! (regular). Each golden is `tests/snapshots/month_grid/<name>.html`, linted and every `ds-`
//! class in it styled by the stylesheet. The month is August 2026 from Monday, six weeks.
//!
//! `DS_BLESS=1 cargo test -p ds --features lint --test it month_grid_density_ssr` rewrites them.

use crate::support::month_sample;
use crate::support::specimens::Specimen;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::prelude::*;
use month_sample::{AUGUST, First, sample};
use sill_shell_kit::month_grid::data::{MonthStep, WeekNumbers};
use sill_shell_kit::month_grid::density::MonthDensity;
use sill_shell_kit::prelude::*;
use sill_shell_kit::tokens::widgets::WidgetMetrics;

/// One specimen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case {
    /// `MonthDensity::Compact` on a Popover, outside any frame, asking for week numbers.
    Compact,
    /// `Auto` in a small desktop frame.
    SmallFrame,
    /// `Auto` in a medium desktop frame.
    MediumFrame,
    LargeFrame,
}

const CASES: [(Case, &str); 4] = [
    (Case::Compact, "compact"),
    (Case::SmallFrame, "auto-small-frame"),
    (Case::MediumFrame, "auto-medium-frame"),
    (Case::LargeFrame, "auto-large-frame"),
];

#[derive(Props, Clone, PartialEq)]
struct CaseProps {
    case: Case,
}

/// The grid with the step buttons and week numbers asked for.
fn grid(density: MonthDensity) -> Element {
    rsx! {
        MonthGrid {
            data: sample(AUGUST, First::Monday),
            weeks: WeekNumbers::Show,
            density,
            onstep: EventHandler::new(|_: MonthStep| {}),
        }
    }
}

/// `size`'s frame on the desktop layer, as sill's desktop surface draws it.
fn framed(size: WidgetSize) -> Element {
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Widget, chrome: Some(RootChrome::Transparent), stylesheet: Inject::Host,
            div { style: WidgetMetrics::default().style_attr(),
                WidgetFrame { size, {grid(MonthDensity::Auto)} }
            }
        }
    }
}

#[allow(non_snake_case)]
fn Specimen(props: CaseProps) -> Element {
    match props.case {
        Case::Compact => rsx! {
            Ds { appearance: Appearance::default(), material: Material::Popover, stylesheet: Inject::Host,
                {grid(MonthDensity::Compact)}
            }
        },
        Case::SmallFrame => framed(WidgetSize::Small),
        Case::MediumFrame => framed(WidgetSize::Medium),
        Case::LargeFrame => framed(WidgetSize::Large),
    }
}

fn render(case: Case) -> String {
    let mut dom = VirtualDom::new_with_props(Specimen, CaseProps { case });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

pub(crate) fn specimens() -> Vec<Specimen> {
    CASES
        .iter()
        .map(|&(case, name)| Specimen::of(&format!("month_grid/{name}.html"), render(case)))
        .collect()
}

/// Compact wherever it resolves so: `data-density=compact`, no week column though the caller
/// asked for one, and the plain step buttons in place of the Tool buttons (a medium card's 140
/// of height is too short for six regular weeks, design/23 section 5.2); the large frame keeps
/// the regular grid, its week numbers and its Tool buttons, and fills the card.
#[test]
fn the_density_resolves_and_says_so() {
    for case in [Case::Compact, Case::SmallFrame, Case::MediumFrame] {
        let html = render(case);
        assert!(html.contains("data-density=\"compact\""), "{case:?}");
        assert!(html.contains("data-weeks=\"hide\""), "{case:?}");
        assert_eq!(
            html.matches("class=\"ds-month-week\"").count(),
            0,
            "{case:?}"
        );
        assert_eq!(
            html.matches("class=\"ds-month-step\"").count(),
            2,
            "{case:?}"
        );
        assert!(!html.contains("ds-icon-button"), "{case:?}");
        assert_eq!(
            html.matches("class=\"ds-month-day\"").count(),
            42,
            "{case:?}"
        );
    }
    let large = render(Case::LargeFrame);
    assert!(large.contains("data-density=\"regular\""));
    assert!(
        large.contains("data-fit=\"frame\""),
        "a framed month fills its card"
    );
    assert!(large.contains("data-weeks=\"show\""));
    assert_eq!(large.matches("class=\"ds-month-week\"").count(), 1 + 6);
    assert_eq!(large.matches("data-variant=\"toolbar\"").count(), 2);
    assert!(!large.contains("ds-month-step"));
    assert!(
        !render(Case::Compact).contains("data-fit"),
        "outside a frame it keeps its size"
    );
}
