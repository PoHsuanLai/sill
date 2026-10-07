//! MonthGrid: a month of days in seven columns, the calendar widget's month (design/04-COMPONENTS.md
//! section 39; design/20-SURFACES.md section 1.12). The shell computes the month;
//! this draws it: a header with the month's title and, when the caller steps months, the
//! previous and next `Button { Tool }`; the weekday heads; then the weeks, optionally led by
//! their ISO week numbers, the neighbours' days quieter, today on the accent disc and a busy
//! day's dot. A change of month slides the weeks in once (`month_grid_weeks`).
//!
//! Two densities: the regular grid, and a compact one that fits a small desktop
//! widget's 140 x 140 content box, which the regular grid (224 x 254 for six weeks) overflows.
//! `MonthDensity::Auto` picks between them by the enclosing `WidgetFrame`.

pub mod data;
pub mod density;
pub(crate) mod header;
pub(crate) mod weeks;

use crate::month_grid::data::{DayKey, MonthGridData, MonthKey, MonthStep, WeekNumbers};
use crate::month_grid::density::{Drawn, MonthDensity};
use crate::month_grid::header::header;
use crate::month_grid::weeks::{MonthSlide, MonthWeeks};
use crate::widget::scope::use_enclosing_frame;
use dioxus::prelude::*;
use ds::base::word::Word;
use ds::components::content::text_runs::{TextLine, text};
use ds::root::common::Common;

/// A month. `data` is the month as the shell laid it out; `weeks` whether each row leads with
/// its ISO week. With `onstep` the header ends in the previous and next buttons, which call it
/// with their `MonthStep`; without it the header is the title alone. With `onpick` every day is a
/// `button` that calls it with the day's key; without it the days are plain text.
///
/// Changing `data.month` slides the new month's weeks in once: from the right for a later
/// month (`slide-r`), from the left for an earlier one (`slide-l`); the first month drawn and
/// a render that keeps the month play nothing.
///
/// `density` (`Auto`) is written as `data-density`: `Auto` draws compact inside a
/// `WidgetFrame { size: Small }` and regular inside a Medium or Large one or outside any frame;
/// `Regular` and `Compact` force it. The compact grid fills its box (in a small frame, the
/// 140 x 140 content box): a 14 px header of `--fs-micro` title and 14 px glyph buttons, 12 px
/// heads, then the weeks sharing the rest evenly, seven columns across (20 each) and the rows
/// down (19.67 for six weeks, 23.6 for five, the weeks running 4 past the box's foot as an
/// optical inset), today on a 20 px disc (twice `--fs-caption`) in the middle of its
/// row and a 3 px dot, measured on Blitz (the `month_grid_density` harness test). It never draws
/// week numbers, whatever `weeks` says: a week column would not fit.
#[component]
pub fn MonthGrid(
    data: MonthGridData,
    #[props(default)] weeks: WeekNumbers,
    #[props(default)] density: MonthDensity,
    #[props(default)] onstep: Option<EventHandler<MonthStep>>,
    #[props(default)] onpick: Option<EventHandler<DayKey>>,
    #[props(default)] common: Common,
) -> Element {
    let slide = use_month_slide(data.month);
    let frame = use_enclosing_frame();
    let drawn = density.resolve(frame);
    // In a widget's card the month fills the content box at either density (design/23
    // section 5.2): the rows share its height, the columns its width.
    let fit = frame.map(|_| "frame");
    let weeks = shown_weeks(weeks, drawn);
    let label = common
        .aria_label
        .clone()
        .unwrap_or_else(|| data.title.plain_text());
    let class = common.class("ds-month");
    let attrs = common.data_attributes();
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-weeks": weeks.slug(),
            "data-density": drawn.slug(),
            "data-fit": fit,
            "aria-label": "{label}",
            onmounted: move |event| common.mounted(event),
            ..attrs,
            {header(&data.title, drawn, onstep)}
            {heads(&data.heads, weeks)}
            // A keyed list of one: a key only remounts inside a list, so a new month is a new
            // body (and a new slide) while a render that keeps the month keeps the body.
            for month in std::iter::once(data.month) {
                MonthWeeks {
                    key: "{month.slug()}",
                    weeks: data.weeks.clone(),
                    numbers: weeks,
                    slide,
                    onpick,
                }
            }
        }
    }
}

/// The weekday heads, behind an empty week-number cell when the numbers show.
fn heads(heads: &[TextLine; 7], weeks: WeekNumbers) -> Element {
    rsx! {
        div { class: "ds-month-row", "data-row": "heads",
            if weeks == WeekNumbers::Show {
                span { class: "ds-month-week" }
            }
            for (at , head) in heads.iter().enumerate() {
                span { key: "{at}", class: "ds-month-head", {text(head)} }
            }
        }
    }
}

/// The week numbers drawn: the caller's, except never at the compact density.
fn shown_weeks(weeks: WeekNumbers, drawn: Drawn) -> WeekNumbers {
    match drawn {
        Drawn::Regular => weeks,
        Drawn::Compact => WeekNumbers::Hide,
    }
}

/// Which way the weeks slide in this render: the order of `month` against the month drawn
/// before, held until the month changes again. The first month drawn is still.
fn use_month_slide(month: MonthKey) -> MonthSlide {
    let mut seen = use_hook(|| CopyValue::new((month, MonthSlide::Still)));
    let (last, slide) = *seen.peek();
    let now = MonthSlide::between(last, month).unwrap_or(slide);
    seen.set((month, now));
    now
}
