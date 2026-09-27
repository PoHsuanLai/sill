//! The Up Next widget on quire's widget contract (kind `sill.up-next`): macOS's Up Next, today's
//! weekday in the accent over the day's number, then the next events, each with its calendar's
//! colour beside its title and time; the view keeps only the rows its card's size holds whole
//! (G291, `super::up_next::events_capacity`). Its entry is plain data (`up_next::up_next_entry`
//! builds it); it has no controls.

use dioxus::prelude::*;
use ds::{Icon, LabelHue, NoIntent, Text, WidgetContext, WidgetHost, WidgetSize, WidgetTitle};
use serde::{Deserialize, Serialize};

use super::up_next::events_capacity;

/// The Up Next widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct UpNext;

/// One moment of Up Next.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpNextEntry {
    /// No agenda read yet: the card alone.
    #[default]
    Waiting,
    /// Today and the next events.
    Agenda(UpNextFace),
}

/// What Up Next shows once the agenda is read.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UpNextFace {
    /// Today.
    pub day: DayLine,
    /// The next events, in order; the view draws what its size holds.
    pub rows: Vec<UpNextRow>,
    /// The line under the events naming a source problem ("Calendars are off").
    pub quiet: Option<String>,
}

/// Today as the day column says it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DayLine {
    /// "Sunday".
    pub weekday: String,
    /// The day of the month.
    pub date: u8,
}

/// One event's row.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UpNextRow {
    /// Its calendar's colour.
    pub hue: LabelHue,
    /// Its title.
    pub title: String,
    /// `All day`, `Now`, `10:00 – 11:30`, `Tomorrow 09:00`.
    pub when: String,
}

impl ds::Widget for UpNext {
    type Entry = UpNextEntry;
    type Intent = NoIntent;

    fn kind() -> ds::WidgetKind {
        ds::WidgetKind::fixed("sill.up-next")
    }

    fn name() -> Text {
        Text::from("Up Next")
    }

    fn description() -> Text {
        Text::from("See your next events from your calendars.")
    }

    fn sizes() -> &'static [WidgetSize] {
        &[WidgetSize::Medium, WidgetSize::Large]
    }

    fn placeholder(_size: WidgetSize) -> UpNextEntry {
        UpNextEntry::Waiting
    }

    fn preview(_size: WidgetSize) -> UpNextEntry {
        let row = |hue, title: &str, when: &str| UpNextRow {
            hue,
            title: title.to_owned(),
            when: when.to_owned(),
        };
        UpNextEntry::Agenda(UpNextFace {
            day: DayLine {
                weekday: "Monday".to_owned(),
                date: 14,
            },
            rows: vec![
                row(LabelHue::Blue, "Design review", "10:00 – 11:00"),
                row(LabelHue::Green, "Lunch with Mei", "13:30 – 14:30"),
                row(LabelHue::Amber, "Climbing", "17:00 – 19:00"),
            ],
            quiet: None,
        })
    }

    fn view(entry: &UpNextEntry, cx: WidgetContext<NoIntent>) -> Element {
        let UpNextEntry::Agenda(face) = entry else {
            return rsx! {};
        };
        let class = match cx.host {
            WidgetHost::Tile => "sill-un sill-un-center",
            WidgetHost::Desktop => "sill-un sill-un-desktop",
        };
        let rows: Vec<UpNextRow> = face
            .rows
            .iter()
            .take(events_capacity(cx.size))
            .cloned()
            .collect();
        let empty = match (rows.is_empty(), &face.quiet) {
            (true, None) => Some("No upcoming events"),
            (true, Some(_)) | (false, _) => None,
        };
        rsx! {
            div { class,
                div { class: "sill-un-day",
                    span { class: "sill-un-weekday", "{face.day.weekday}" }
                    span { class: "sill-un-date", "{face.day.date}" }
                }
                div { class: "sill-un-list",
                    if let Some(empty) = empty {
                        span { class: "sill-un-empty", {empty} }
                    }
                    for row in rows {
                        div { class: "sill-un-row",
                            span { class: "sill-un-bar", "data-hue": row.hue.slug() }
                            div { class: "sill-un-text",
                                span { class: "sill-un-title ds-truncate", {row.title} }
                                span { class: "sill-un-when", {row.when} }
                            }
                        }
                    }
                    if let Some(quiet) = &face.quiet {
                        span { class: "sill-un-quiet", "{quiet}" }
                    }
                }
            }
        }
    }

    fn title() -> Option<WidgetTitle> {
        Some(WidgetTitle::new(Icon::Clock, "Up Next"))
    }
}
