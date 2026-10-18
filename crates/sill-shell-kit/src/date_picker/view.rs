//! DatePicker (design/30 section 2.3, `NSDatePicker`). The textual style is one bezel of
//! segments (month, day, year, hour, minute) with a stepper after it: a press selects a segment,
//! Up and Down or the stepper change it, Left and Right move between segments. The graphical
//! style is the `MonthGrid`, its header stepping months and a press on a day picking it, above
//! a time field of the same segments when the picker holds a time. The day picked is the grid's
//! accent disc.
//!
//! Markup: `span.ds-date-picker[data-style][data-size]` of `span.ds-date-picker-field`
//! (`button.ds-date-picker-segment[data-selected]` and `span.ds-date-picker-sep`) and a `Stepper`
//! (textual), or `div.ds-date-picker[data-style=graphical]` of a `MonthGrid` and
//! `div.ds-date-picker-time`.

use crate::date_picker::calendar::{month_after, month_grid};
use crate::date_picker::model::{DateValue, Elements, PickerStyle, Segment};
use crate::month_grid::MonthGrid;
use crate::month_grid::data::{MonthKey, MonthStep, WeekNumbers};
use crate::month_grid::density::MonthDensity;
use dioxus::prelude::*;
use ds::components::fields::stepper::model::{Readout, StepDirection};
use ds::components::fields::stepper::view::Stepper;
use ds::prelude::*;
use ds::root::common::Common;
use ds::style::tokens::control_size::ControlSize;

/// The segments `elements` shows, in reading order.
fn segments(elements: Elements) -> Vec<Segment> {
    match elements {
        Elements::Date => vec![Segment::Month, Segment::Day, Segment::Year],
        Elements::Time => vec![Segment::Hour, Segment::Minute],
        Elements::DateAndTime => vec![
            Segment::Month,
            Segment::Day,
            Segment::Year,
            Segment::Hour,
            Segment::Minute,
        ],
    }
}

/// What a segment reads: two digits, the year four.
fn digits(value: DateValue, segment: Segment) -> String {
    match segment {
        Segment::Year => format!("{:04}", value.value(segment)),
        Segment::Month | Segment::Day | Segment::Hour | Segment::Minute => {
            format!("{:02}", value.value(segment))
        }
    }
}

/// The word between a segment and the next.
fn separator(after: Segment) -> Option<&'static str> {
    match after {
        Segment::Month | Segment::Day => Some("/"),
        Segment::Hour => Some(":"),
        Segment::Year => Some(" "),
        Segment::Minute => None,
    }
}

/// The segments as buttons, `selected` marked, each reporting a press to `select`.
fn field(
    value: DateValue,
    shown: &[Segment],
    selected: Segment,
    availability: Availability,
    select: EventHandler<Segment>,
) -> Element {
    let live = availability == Availability::Enabled;
    rsx! {
        span { class: "ds-date-picker-field", role: "group",
            for (at , segment) in shown.iter().copied().enumerate() {
                button {
                    key: "{segment.slug()}",
                    r#type: "button",
                    class: "ds-date-picker-segment",
                    role: "spinbutton",
                    "aria-label": segment.label(),
                    "aria-valuenow": "{value.value(segment)}",
                    "aria-valuemin": "{value.range(segment).min()}",
                    "aria-valuemax": "{value.range(segment).max()}",
                    "data-selected": if segment == selected { Some("true") } else { None },
                    disabled: if live { None } else { Some("true") },
                    onclick: move |_| select.call(segment),
                    "{digits(value, segment)}"
                }
                if let Some(word) = separator(segment).filter(|_| at + 1 < shown.len()) {
                    span { class: "ds-date-picker-sep", "aria-hidden": "true", "{word}" }
                }
            }
        }
    }
}

/// A date picker holding `value`; `onchange` hears the whole date and time after each change.
#[component]
pub fn DatePicker(
    #[props(into)] label: String,
    value: DateValue,
    #[props(default)] style: PickerStyle,
    #[props(default)] elements: Elements,
    #[props(default)] size: ControlSize,
    #[props(default)] availability: Availability,
    onchange: EventHandler<DateValue>,
    #[props(default)] common: Common,
) -> Element {
    let mut selected = use_signal(|| None::<Segment>);
    let mut viewing = use_signal(|| None::<MonthKey>);
    let live = availability == Availability::Enabled;
    let time_only = segments(Elements::Time);
    let shown = match style {
        PickerStyle::Textual => segments(elements),
        PickerStyle::Graphical => time_only,
    };
    let current = selected()
        .filter(|segment| shown.contains(segment))
        .or_else(|| shown.first().copied())
        .unwrap_or(Segment::Month);
    let stepper = rsx! {
        Stepper {
            label: current.label(),
            value: value.value(current),
            range: value.range(current),
            readout: Readout::Bare,
            size,
            availability,
            onchange: move |to| onchange.call(value.with(current, to)),
        }
    };
    let keys = shown.clone();
    let on_key = move |event: KeyboardEvent| {
        if !live {
            return;
        }
        let at = keys
            .iter()
            .position(|segment| *segment == current)
            .unwrap_or(0);
        match event.key() {
            Key::ArrowUp => onchange.call(value.stepped(current, StepDirection::Up)),
            Key::ArrowDown => onchange.call(value.stepped(current, StepDirection::Down)),
            Key::ArrowLeft => selected.set(keys.get(at.saturating_sub(1)).copied()),
            Key::ArrowRight => selected.set(keys.get(at + 1).or_else(|| keys.last()).copied()),
            _ => return,
        }
        event.prevent_default();
    };
    let month = viewing().unwrap_or(MonthKey {
        year: value.day.year,
        month: value.day.month,
    });
    let data = common.data_attributes();
    let class = common.class("ds-date-picker");
    let has_time = elements != Elements::Date;
    rsx! {
        span {
            id: common.id.clone(),
            class,
            "data-style": style.slug(),
            "data-size": size.slug(),
            "data-availability": availability.slug(),
            "aria-label": common.aria_label.clone().unwrap_or(label),
            "aria-disabled": availability.aria_disabled(),
            onmounted: move |event| common.mounted(event),
            onkeydown: on_key,
            ..data,
            match style {
                PickerStyle::Textual => rsx! {
                    {field(value, &shown, current, availability, EventHandler::new(move |segment| selected.set(Some(segment))))}
                    {stepper}
                },
                PickerStyle::Graphical => rsx! {
                    if elements != Elements::Time {
                        MonthGrid {
                            data: month_grid(month, value.day),
                            weeks: WeekNumbers::Hide,
                            density: MonthDensity::Regular,
                            onstep: move |step: MonthStep| {
                                let by = match step {
                                    MonthStep::Previous => -1,
                                    MonthStep::Next => 1,
                                };
                                viewing.set(Some(month_after(month, by)));
                            },
                            onpick: move |day| {
                                if live {
                                    viewing.set(None);
                                    onchange.call(DateValue { day, ..value });
                                }
                            },
                        }
                    }
                    if has_time {
                        div { class: "ds-date-picker-time",
                            {field(value, &shown, current, availability, EventHandler::new(move |segment| selected.set(Some(segment))))}
                            {stepper}
                        }
                    }
                },
            }
        }
    }
}
