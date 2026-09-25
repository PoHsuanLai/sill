//! What the Up Next widget lists, from an agenda: pure over the time the caller read.

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};

use super::model::{CalEvent, EventTime};

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
    let today = now.to_zoned(zone.clone()).date();
    let horizon = today.saturating_add(i64::from(ahead.0).saturating_add(1).days());
    let horizon_at = midnight(horizon, zone);
    let mut left: Vec<(Place, &CalEvent)> = events
        .iter()
        .filter(|event| match event.when {
            EventTime::AllDay { first, last } => last >= today && first < horizon,
            EventTime::Timed { start, end } => {
                let still = end > now || (end == start && start >= now);
                still && horizon_at.is_none_or(|horizon| start < horizon)
            }
        })
        .map(|event| (place(event, today, zone), event))
        .collect();
    left.sort_by(|(a, x), (b, y)| a.cmp(b).then_with(|| x.title.cmp(&y.title)));
    left.into_iter()
        .take(max)
        .map(|(_, event)| event.clone())
        .collect()
}

/// Where an event sorts in Up Next: its day in the zone (today for one under way since
/// earlier), all-day before timed, then its start.
type Place = (Date, u8, Timestamp);

fn place(event: &CalEvent, today: Date, zone: &TimeZone) -> Place {
    match event.when {
        EventTime::AllDay { first, .. } => (first.max(today), 0, Timestamp::MIN),
        EventTime::Timed { start, .. } => {
            (start.to_zoned(zone.clone()).date().max(today), 1, start)
        }
    }
}

/// The instant `day` starts in `zone`.
fn midnight(day: Date, zone: &TimeZone) -> Option<Timestamp> {
    day.to_zoned(zone.clone())
        .ok()
        .map(|zoned| zoned.timestamp())
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
