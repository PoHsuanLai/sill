//! What the Up Next widget lists, from an agenda: pure over the time the caller read.

use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::model::CalEvent;

/// A number of days.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Days(pub u16);

/// The events still to come or under way at `now`, starting before the end of the day `ahead`
/// days after today (in `zone`), in start order, all-day events first on their day, at most
/// `max`. An all-day event counts as under way for the whole of its days in `zone`.
pub fn up_next(
    events: &[CalEvent],
    now: Timestamp,
    zone: &TimeZone,
    ahead: Days,
    max: usize,
) -> Vec<CalEvent> {
    let _ = (events, now, zone, ahead, max);
    todo!("M10 fill: a-calendar")
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use jiff::tz::TimeZone;

    use super::{Days, up_next};
    use crate::events::model::{CalEvent, CalendarName, EventTime, EventUid};

    fn event(uid: &str, when: EventTime) -> CalEvent {
        CalEvent {
            uid: EventUid(uid.to_owned()),
            title: uid.to_owned(),
            when,
            location: None,
            calendar: CalendarName("std".to_owned()),
        }
    }

    #[test]
    #[ignore = "M10 fill: a-calendar"]
    fn up_next_lists_what_is_left_in_order() {
        let utc = TimeZone::UTC;
        let at = |d, h| {
            date(2026, 9, d)
                .at(h, 0, 0, 0)
                .to_zoned(utc.clone())
                .map(|z| z.timestamp())
                .unwrap_or_else(|e| panic!("{e}"))
        };
        let timed = |uid, d, from, to| {
            event(
                uid,
                EventTime::Timed {
                    start: at(d, from),
                    end: at(d, to),
                },
            )
        };
        let day = |uid, d| {
            event(
                uid,
                EventTime::AllDay {
                    first: date(2026, 9, d),
                    last: date(2026, 9, d),
                },
            )
        };
        let events = vec![
            timed("over", 26, 8, 9),
            timed("now", 26, 9, 11),
            day("today", 26),
            timed("later", 27, 9, 10),
            timed("far", 30, 9, 10),
        ];
        let now = at(26, 10);
        let names = |got: Vec<_>| {
            got.into_iter()
                .map(|e: super::CalEvent| e.uid.0)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(up_next(&events, now, &utc, Days(1), 10)),
            vec!["today", "now", "later"]
        );
        assert_eq!(
            names(up_next(&events, now, &utc, Days(7), 2)),
            vec!["today", "now"]
        );
        assert_eq!(names(up_next(&events, now, &utc, Days(7), 10)).len(), 4);
    }
}
