//! DatePicker as markup (design/30 section 2.3): the goldens are
//! `tests/snapshots/date_picker/<name>.html`, each linted against the stylesheet and every
//! `ds-date-picker` class in it styled by the picker's sheet.
//!
//! `DS_BLESS=1 cargo test -p sill-shell-kit --test it date_picker_ssr` rewrites the goldens.

use crate::support::golden;

use dioxus::prelude::*;
use ds::assembly::ds::Inject;
use ds::prelude::*;
use ds_lint::{LintConfig, markup};
use sill_shell_kit::date_picker::model::{DateValue, Elements, PickerStyle, TimeOfDay};
use sill_shell_kit::month_grid::data::DayKey;
use sill_shell_kit::prelude::*;

const PICKER_CSS: &str = include_str!("../../src/date_picker/style.css");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Case {
    Textual,
    DateAndTime,
    Graphical,
    Disabled,
}

const CASES: [(Case, &str); 4] = [
    (Case::Textual, "textual"),
    (Case::DateAndTime, "textual-date-and-time"),
    (Case::Graphical, "graphical"),
    (Case::Disabled, "disabled"),
];

#[derive(Props, Clone, PartialEq)]
struct CaseProps {
    case: Case,
}

fn start() -> DateValue {
    DateValue {
        day: DayKey {
            year: 2026,
            month: 9,
            day: 30,
        },
        time: TimeOfDay {
            hour: 14,
            minute: 41,
        },
    }
}

#[allow(non_snake_case)]
fn Specimen(props: CaseProps) -> Element {
    let picker = match props.case {
        Case::Textual => rsx! { DatePicker { label: "Date", value: start(), onchange: |_| {} } },
        Case::DateAndTime => {
            rsx! { DatePicker { label: "Date", value: start(), elements: Elements::DateAndTime, onchange: |_| {} } }
        }
        Case::Graphical => {
            rsx! { DatePicker { label: "Date", value: start(), style: PickerStyle::Graphical, elements: Elements::DateAndTime, onchange: |_| {} } }
        }
        Case::Disabled => {
            rsx! { DatePicker { label: "Date", value: start(), availability: Availability::Disabled, onchange: |_| {} } }
        }
    };
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Popover, stylesheet: Inject::Host, {picker} }
    }
}

fn render(case: Case) -> String {
    let mut dom = VirtualDom::new_with_props(Specimen, CaseProps { case });
    dom.rebuild_in_place();
    dioxus_ssr::render(&dom)
}

#[test]
fn every_state_matches_its_golden() {
    let failures: Vec<String> = CASES
        .iter()
        .filter_map(|&(case, name)| {
            golden::check(&format!("date_picker/{name}.html"), &render(case)).err()
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_state_lints_clean_and_its_picker_classes_are_styled() {
    let config = LintConfig::new(&sill_shell_kit::kits());
    let css = sill_shell_kit::stylesheet();
    let mut failures = Vec::new();
    for (case, name) in CASES {
        let html = render(case);
        for offence in markup(&html, css, &config) {
            failures.push(format!("{name}: {:?} {}", offence.rule, offence.text));
        }
        for class in html
            .split("class=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .flat_map(str::split_whitespace)
        {
            if class.starts_with("ds-date-picker") && !PICKER_CSS.contains(&format!(".{class}")) {
                failures.push(format!("{name}: .{class} is not in the picker's sheet"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
