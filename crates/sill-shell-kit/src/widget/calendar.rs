//! The month widget on the widget contract (design/23-WIDGETS.md sections 5, 5.2 and 9.6), laid
//! out as the reference's calendar widgets are rather than one grid stretched over the card:
//! Small, the compact month filling the card at its own pitch; Medium, a today column (the
//! weekday in the accent, the date large, the next event or the quiet line) beside the compact
//! month at the small card's metrics; Large, the regular month at its own pitch, centred, over
//! the day's events. The header's step buttons are the widget's one control: they send
//! [`MonthIntent::Step`], and the provider answers with the next month's timeline.

use crate::month_grid::MonthGrid;
use crate::month_grid::data::{MonthGridData, MonthStep, WeekNumbers};
use crate::widget::contract::{Widget, WidgetContext, WidgetKind};
use crate::widget::kind::WidgetSize;
use dioxus::prelude::*;
use ds::base::word::Word;
use ds::style::tokens::label_hue::LabelHue;
use serde::{Deserialize, Serialize};

/// The month widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MonthWidget;

/// One moment of the month widget.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonthEntry {
    /// No month yet: the card alone.
    #[default]
    Waiting,
    /// The month, today and the day's events.
    Month(Box<MonthFace>),
}

/// What the month widget shows once its provider has spoken.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonthFace {
    /// The grid, in the provider's words.
    pub grid: MonthGridData,
    /// `calendar.week_numbers`.
    pub weeks: WeekNumbers,
    /// Today, for the Medium card's today column; none draws the column without it.
    #[serde(default)]
    pub today: Option<TodayLine>,
    /// Today's events still to come, in order: the Medium card shows the first, the Large card
    /// as many as fit. The Calendar app fills them; sill's Up Next can feed them now.
    #[serde(default)]
    pub events: Vec<EventLine>,
    /// The quiet line when there are none, in the provider's words ("No events today").
    #[serde(default)]
    pub no_events: String,
}

impl MonthFace {
    /// `grid` with week numbers as `weeks`, no today column and no events.
    pub fn of(grid: MonthGridData, weeks: WeekNumbers) -> Self {
        MonthFace {
            grid,
            weeks,
            today: None,
            events: Vec::new(),
            no_events: String::new(),
        }
    }
}

/// Today as the Medium card's today column says it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TodayLine {
    /// The weekday in the provider's words ("Sunday").
    pub weekday: String,
    /// The day of the month.
    pub day: u8,
}

/// One event in the widget.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventLine {
    /// Its time in the provider's words ("10:00", "All day").
    pub time: String,
    /// Its title.
    pub title: String,
    /// Its calendar's colour.
    pub hue: LabelHue,
}

/// What the month widget's controls ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MonthIntent {
    /// Show the month before or after.
    Step(MonthStep),
}

impl Widget for MonthWidget {
    type Entry = MonthEntry;
    type Intent = MonthIntent;

    fn kind() -> WidgetKind {
        WidgetKind::fixed("quire.month")
    }

    fn sizes() -> &'static [WidgetSize] {
        &[WidgetSize::Small, WidgetSize::Medium, WidgetSize::Large]
    }

    fn placeholder(_size: WidgetSize) -> MonthEntry {
        MonthEntry::Waiting
    }

    fn view(entry: &MonthEntry, cx: WidgetContext<MonthIntent>) -> Element {
        let MonthEntry::Month(face) = entry else {
            return rsx! {};
        };
        let onstep = cx
            .act
            .map(|act| EventHandler::new(move |step: MonthStep| act.call(MonthIntent::Step(step))));
        let month = rsx! {
            MonthGrid { data: face.grid.clone(), weeks: face.weeks, onstep }
        };
        match cx.size {
            WidgetSize::Small => month,
            WidgetSize::Medium => rsx! {
                div { class: "ds-month-widget", "data-layout": "split",
                    {today_column(face)}
                    div { class: "ds-month-widget-month", {month} }
                }
            },
            WidgetSize::Large => rsx! {
                div { class: "ds-month-widget", "data-layout": "stack",
                    {month}
                    {event_list(face)}
                }
            },
        }
    }
}

/// How many events the Large card lists under the month.
pub const LARGE_EVENTS: usize = 3;

/// The Medium card's left half: the weekday, the date, the next event or the quiet line.
fn today_column(face: &MonthFace) -> Element {
    let next = face.events.first();
    rsx! {
        div { class: "ds-month-today",
            if let Some(today) = &face.today {
                span { class: "ds-month-weekday", "{today.weekday}" }
                span { class: "ds-month-date", "{today.day}" }
            }
            match next {
                Some(event) => event_row(event),
                None => quiet(&face.no_events),
            }
        }
    }
}

/// The Large card's foot: the day's events, or the quiet line.
fn event_list(face: &MonthFace) -> Element {
    rsx! {
        div { class: "ds-month-events",
            if face.events.is_empty() {
                {quiet(&face.no_events)}
            }
            for event in face.events.iter().take(LARGE_EVENTS) {
                {event_row(event)}
            }
        }
    }
}

/// One event: its calendar's bar, its title, its time.
fn event_row(event: &EventLine) -> Element {
    rsx! {
        div { class: "ds-month-event", "data-hue": event.hue.slug(),
            span { class: "ds-month-event-title", "{event.title}" }
            span { class: "ds-month-event-time", "{event.time}" }
        }
    }
}

/// The quiet line, or nothing when the provider gave no words.
fn quiet(words: &str) -> Element {
    if words.is_empty() {
        return rsx! {};
    }
    rsx! {
        span { class: "ds-month-quiet", "{words}" }
    }
}
