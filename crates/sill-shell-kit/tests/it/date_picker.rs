//! DatePicker on a real Blitz document (design/30 section 2.3): a press selects a segment, Up
//! steps it and December wraps to January; in the graphical style a press on a day picks it and
//! the header steps months.

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::{Clock, Harness, HarnessConfig, Input, Viewport};
use sill_shell_kit::date_picker::model::{DateValue, Elements, PickerStyle, TimeOfDay};
use sill_shell_kit::month_grid::data::DayKey;
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 400,
    height: 400,
    scale_percent: 100,
};

fn start() -> DateValue {
    DateValue {
        day: DayKey {
            year: 2026,
            month: 12,
            day: 14,
        },
        time: TimeOfDay { hour: 9, minute: 5 },
    }
}

#[allow(non_snake_case)]
fn Textual() -> Element {
    let mut value = use_signal(start);
    let said = format!("{:?}", value().day);
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            DatePicker { label: "Date", value: value(), onchange: move |next| value.set(next) }
            p { class: "day", "{said}" }
        }
    }
}

#[allow(non_snake_case)]
fn Graphical() -> Element {
    let mut value = use_signal(start);
    let said = format!("{:?}", value().day);
    rsx! {
        Ds { appearance: Appearance::default(), material: Material::Window,
            DatePicker { label: "Date", value: value(), style: PickerStyle::Graphical, elements: Elements::Date, onchange: move |next| value.set(next) }
            p { class: "day", "{said}" }
        }
    }
}

fn harness(page: fn() -> Element) -> Harness {
    let mut harness = Harness::new(page, HarnessConfig::new(VIEW).with_clock(Clock::Virtual));
    harness.advance(Duration::from_millis(100));
    harness
}

#[test]
fn a_selected_segment_steps_and_december_wraps_to_january() {
    let mut harness = harness(Textual);
    let month = harness
        .centre(".ds-date-picker-segment")
        .expect("month segment");
    harness.send(Input::click(month));
    harness.send(Input::key(ShortcutKey::Up));
    assert_eq!(
        harness.text_of(".day").as_deref(),
        Some("DayKey { year: 2026, month: 1, day: 14 }")
    );
}

#[test]
fn a_press_on_a_day_picks_it_and_the_arrow_shows_the_next_month() {
    let mut harness = harness(Graphical);
    let day = harness
        .centre(".ds-month-row:nth-child(3) button:nth-child(3)")
        .expect("a day");
    harness.send(Input::click(day));
    let said = harness.text_of(".day").unwrap_or_default();
    assert!(
        said.contains("month: 12") && !said.contains("day: 14 "),
        "{said}"
    );
    let title = |harness: &Harness| harness.text_of(".ds-month-title").unwrap_or_default();
    assert_eq!(title(&harness), "December 2026");
    let next = harness
        .centre(".ds-month-header .ds-button:last-child")
        .expect("next month");
    harness.send(Input::click(next));
    harness.advance(Duration::from_millis(600));
    assert_eq!(title(&harness), "January 2027");
}
