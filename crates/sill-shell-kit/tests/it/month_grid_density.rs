//! The compact MonthGrid on a real Blitz document (design/04-COMPONENTS.md section 39):
//! at `MonthDensity::Auto` inside a small desktop `WidgetFrame`, the month fills the
//! frame's content box (164 less 12 padding a side, 140 x 140, the widgets' measured inset,
//! design/23 section 1.1), where the regular grid (224 x 254) overflowed it: seven even 20 px
//! columns, and the weeks sharing the height left under the header and heads, so a five-week
//! month's rows are taller than a six-week one's (the compact-spacing pass, 2026-09-27); today's
//! disc is round and roomy enough for two digits. A layout, not a timing, so one look.

use crate::support::month_sample;

use dioxus::prelude::*;
use ds::prelude::*;
use ds_harness::{Clock, Harness, HarnessConfig, Viewport};
use month_sample::{AUGUST, First, SEPTEMBER, sample};
use sill_shell_kit::month_grid::data::{MonthStep, WeekNumbers};
use sill_shell_kit::prelude::*;
use std::time::Duration;

const VIEW: Viewport = Viewport {
    width: 320,
    height: 320,
    scale_percent: 100,
};

/// A small desktop widget holding the month, as sill's calendar widget draws it.
#[allow(non_snake_case)]
fn SmallCalendar() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Widget, chrome: Some(RootChrome::Transparent),
            div {
                WidgetFrame { size: WidgetSize::Small,
                    MonthGrid {
                        data: sample(AUGUST, First::Monday),
                        weeks: WeekNumbers::Show,
                        onstep: move |_: MonthStep| {},
                    }
                }
            }
        }
    }
}

fn rect(harness: &Harness, selector: &str) -> Rect {
    harness
        .rect(selector)
        .unwrap_or_else(|| panic!("{selector} is missing:\n{}", harness.html()))
}

/// Whether `inner` lies within `outer`, to a hundredth of a pixel.
fn within(inner: Rect, outer: Rect) -> bool {
    let slack = 0.01;
    inner.origin.x.0 >= outer.origin.x.0 - slack
        && inner.origin.y.0 >= outer.origin.y.0 - slack
        && inner.origin.x.0 + inner.size.width.0 <= outer.origin.x.0 + outer.size.width.0 + slack
        && inner.origin.y.0 + inner.size.height.0 <= outer.origin.y.0 + outer.size.height.0 + slack
}

#[test]
fn the_compact_grid_fills_a_small_widget() {
    let mut harness = Harness::new(
        SmallCalendar,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(Duration::from_millis(50));
    assert_eq!(
        harness.attr(".ds-month", "data-density").as_deref(),
        Some("compact")
    );
    let card = rect(&harness, ".ds-widget");
    let body = rect(&harness, ".ds-widget-body");
    let grid = rect(&harness, ".ds-month");
    let last = rect(&harness, ".ds-month-weeks");
    assert_eq!(
        (body.size.width.0, body.size.height.0),
        (140.0, 140.0),
        "the small frame's content box is 164 less 12 a side"
    );
    assert!(
        (body.origin.x.0 - card.origin.x.0 - 12.0).abs() < 0.01
            && (body.origin.y.0 - card.origin.y.0 - 12.0).abs() < 0.01,
        "the body is the card's content box: {card:?} {body:?}"
    );
    assert_eq!(
        harness.html().matches("class=\"ds-month-row\"").count(),
        1 + 6,
        "the heads and six weeks"
    );
    // The month fills the content box: the header, the heads, then the weeks sharing the rest.
    assert_eq!(
        (grid.size.width.0, grid.size.height.0),
        (140.0, 140.0),
        "the month is the frame's content box"
    );
    let header = rect(&harness, ".ds-month-header");
    let heads = rect(&harness, ".ds-month-row[*|data-row=heads]");
    for (name, part) in [("header", header), ("heads", heads)] {
        assert!(within(part, body), "{name} {part:?} inside {body:?}");
    }
    assert_eq!(
        (
            header.size.height.0,
            heads.size.height.0,
            heads.size.width.0
        ),
        (14.0, 12.0, 140.0),
        "a 14 header, 12 heads, seven 20 columns"
    );
    assert!(
        within(rect(&harness, ".ds-month-step"), body),
        "the step buttons inside the body"
    );
    // The weeks run 4 past the content box's foot (the optical inset: the last row's number sits
    // as far from the card's foot as the title from its top), still inside the card.
    let foot = body.origin.y.0 + body.size.height.0;
    assert!(
        (last.origin.y.0 + last.size.height.0 - foot - 4.0).abs() < 0.01,
        "the weeks end 4 under the body: {last:?} {body:?}"
    );
    assert!(within(last, card), "{last:?} inside the card {card:?}");
    let pitches = week_pitches(&harness, 6);
    for pitch in &pitches {
        assert!(
            (pitch - 118.0 / 6.0).abs() < 0.01,
            "six weeks share 140 - 14 - 12 + 4 = 118 evenly: {pitches:?}"
        );
    }
    let disc = rect(&harness, ".ds-month-weeks > :last-child .ds-month-num");
    assert!(
        within(disc, card),
        "the last week's disc {disc:?} inside {card:?}"
    );
    let day = rect(&harness, ".ds-month-weeks > :first-child > :first-child");
    assert!(
        (day.size.width.0 - 20.0).abs() < 0.01,
        "a column is 140 / 7: {day:?}"
    );
}

/// The heights of the first `count` week rows.
fn week_pitches(harness: &Harness, count: usize) -> Vec<f32> {
    (1..=count)
        .map(|at| {
            rect(harness, &format!(".ds-month-weeks > :nth-child({at})"))
                .size
                .height
                .0
        })
        .collect()
}

/// The five-week September: the same header and heads, and five rows sharing the 118 the six
/// weeks share, each 23.6 against the six-week month's 19.67.
#[test]
fn a_five_week_month_spreads_its_rows_over_the_height() {
    let mut harness = Harness::new(
        SmallSeptember,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(Duration::from_millis(50));
    assert_eq!(harness.count(".ds-month-weeks > .ds-month-row"), 5);
    let pitches = week_pitches(&harness, 5);
    for pitch in &pitches {
        assert!(
            (pitch - 118.0 / 5.0).abs() < 0.01,
            "five weeks share 118 evenly: {pitches:?}"
        );
    }
    let body = rect(&harness, ".ds-widget-body");
    let last = rect(&harness, ".ds-month-weeks");
    assert!(
        (last.origin.y.0 + last.size.height.0 - body.origin.y.0 - body.size.height.0 - 4.0).abs()
            < 0.01,
        "the weeks reach the same optical foot as six do: {last:?} {body:?}"
    );
    // The day's number sits in the middle of its row, the disc with it.
    let cell = rect(&harness, ".ds-month-day[*|aria-current=date]");
    let disc = rect(&harness, ".ds-month-day[*|aria-current=date] .ds-month-num");
    let (mid_cell, mid_disc) = (
        cell.origin.y.0 + cell.size.height.0 / 2.0,
        disc.origin.y.0 + disc.size.height.0 / 2.0,
    );
    assert!(
        (mid_cell - mid_disc).abs() < 0.01,
        "today's disc centred in its row: {cell:?} {disc:?}"
    );
}

/// The same widget on September, whose today (the 26th) is two digits.
#[allow(non_snake_case)]
fn SmallSeptember() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Widget, chrome: Some(RootChrome::Transparent),
            div {
                WidgetFrame { size: WidgetSize::Small,
                    MonthGrid { data: sample(SEPTEMBER, First::Monday), onstep: move |_: MonthStep| {} }
                }
            }
        }
    }
}

/// The 16 px disc left two tabular digits (about 12 px at 700) touching its rim. Today's
/// disc is a circle twice the number's size (`--fs-caption` 10), so two digits keep about 4 px
/// a side, as the regular grid's 24 px disc does round its 11.5 px number.
#[test]
fn todays_compact_disc_is_a_circle_twice_its_number() {
    let mut harness = Harness::new(
        SmallSeptember,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    harness.advance(Duration::from_millis(50));
    assert_eq!(
        harness
            .text_of(".ds-month-day[*|aria-current=date] .ds-month-num")
            .as_deref(),
        Some("26"),
        "today is two digits"
    );
    let disc = rect(&harness, ".ds-month-day[*|aria-current=date] .ds-month-num");
    assert_eq!(
        (disc.size.width.0, disc.size.height.0),
        (20.0, 20.0),
        "twice --fs-caption, round"
    );
    let cell = rect(&harness, ".ds-month-day[*|aria-current=date]");
    assert!(
        disc.origin.x.0 >= cell.origin.x.0 - 0.01
            && disc.origin.x.0 + disc.size.width.0 <= cell.origin.x.0 + cell.size.width.0 + 0.01,
        "the disc {disc:?} stays inside its column {cell:?}"
    );
}

/// The small widget with the month inside a row flex wrapper that does not stretch it, as sill's
/// `.sill-cal` places it: unstretched, the month's own height must still fit the content box.
#[allow(non_snake_case)]
fn WrappedSmallCalendar() -> Element {
    rsx! {
        Ds { sheet: Some(sill_shell_kit::stylesheet()), appearance: Appearance::default(), material: Material::Widget, chrome: Some(RootChrome::Transparent),
            div {
                WidgetFrame { size: WidgetSize::Small,
                    div { style: "display:flex; justify-content:center;",
                        MonthGrid {
                            data: sample(AUGUST, First::Monday),
                            weeks: WeekNumbers::Show,
                            onstep: move |_: MonthStep| {},
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn an_unstretched_six_week_month_still_fits_the_small_widget() {
    let harness = Harness::new(
        WrappedSmallCalendar,
        HarnessConfig::new(VIEW).with_clock(Clock::Virtual),
    );
    let body = rect(&harness, ".ds-widget-body");
    let month = rect(&harness, ".ds-month");
    assert!(
        within(month, body),
        "a six-week month in a wrapper that does not stretch it overflows the content box: \
         month {month:?}, body {body:?}"
    );
}
