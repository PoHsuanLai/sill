//! The Up Next widget (M10 lane a): the next events (`sill_services::events::up_next` over
//! `calendar.up_next_days` and `calendar.up_next_max`), each a title and a time line; "No
//! upcoming events" when there are none, and a quiet line naming the source state when the
//! calendars are off or unreadable. Redrawn on the clock's minute tick.

use dioxus::prelude::*;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use sill_services::CalEvent;

use super::WidgetHost;

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../style/widget_up_next.css");

/// What one event's row says.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventLines {
    /// The title, "Untitled" when empty.
    pub title: String,
    /// `All day`, `Now`, `10:00 – 11:30`, `Tomorrow 09:00`, `Mon 09:00`: 24-hour, like the bar
    /// clock (a 12-hour form has no settings key yet).
    pub when: String,
}

/// `event`'s lines as seen at `now` in `zone`.
pub fn event_lines(event: &CalEvent, now: Timestamp, zone: &TimeZone) -> EventLines {
    let _ = (event, now, zone);
    todo!("M10 fill: a-calendar")
}

/// The next events.
#[component]
pub fn UpNextWidget(host: WidgetHost) -> Element {
    let _ = host;
    rsx! {}
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;
    use sill_services::events::{CalendarName, EventUid};
    use sill_services::{CalEvent, EventTime};

    use super::{EventLines, event_lines};

    #[test]
    #[ignore = "M10 fill: a-calendar"]
    fn an_events_time_line_reads_from_now() {
        let utc = TimeZone::UTC;
        let at = |d, h, m| {
            date(2026, 9, d)
                .at(h, m, 0, 0)
                .to_zoned(utc.clone())
                .map(|z| z.timestamp())
                .unwrap_or_else(|e| panic!("{e}"))
        };
        let event = |title: &str, when| CalEvent {
            uid: EventUid("x".to_owned()),
            title: title.to_owned(),
            when,
            location: None,
            calendar: CalendarName("std".to_owned()),
        };
        let now = at(26, 10, 0);
        let lines = |title: &str, when: &str| EventLines {
            title: title.to_owned(),
            when: when.to_owned(),
        };
        let cases = [
            (
                event(
                    "Standup",
                    EventTime::Timed {
                        start: at(26, 10, 30),
                        end: at(26, 11, 0),
                    },
                ),
                lines("Standup", "10:30 – 11:00"),
            ),
            (
                event(
                    "",
                    EventTime::Timed {
                        start: at(26, 9, 30),
                        end: at(26, 11, 0),
                    },
                ),
                lines("Untitled", "Now"),
            ),
            (
                event(
                    "Dentist",
                    EventTime::Timed {
                        start: at(27, 9, 0),
                        end: at(27, 10, 0),
                    },
                ),
                lines("Dentist", "Tomorrow 09:00"),
            ),
            (
                event(
                    "Holiday",
                    EventTime::AllDay {
                        first: date(2026, 9, 26),
                        last: date(2026, 9, 26),
                    },
                ),
                lines("Holiday", "All day"),
            ),
            (
                event(
                    "Trip",
                    EventTime::Timed {
                        start: at(28, 9, 0),
                        end: at(28, 10, 0),
                    },
                ),
                lines("Trip", "Mon 09:00"),
            ),
        ];
        for (event, want) in cases {
            assert_eq!(event_lines(&event, now, &utc), want, "{}", event.title);
        }
    }
}
