//! `RRULE`, the common subset, and turning a parsed event into the occurrences a window holds.
//! `BYMONTHDAY`, `BYSETPOS`, `BYHOUR` and the like are not expanded: an event whose rule uses
//! them shows its first occurrence only (the reader's problem list says so).

use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;

use super::ics::{IcsEvent, IcsTime};
use super::model::{CalEvent, CalendarName, Window};

/// `FREQ`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// Where a rule stops.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuleEnd {
    Never,
    /// `COUNT`: this many occurrences, the first included.
    Count(u32),
    /// `UNTIL`, inclusive.
    Until(IcsTime),
}

/// One `RRULE`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule {
    pub freq: Freq,
    /// `INTERVAL`, 1 when unsaid.
    pub interval: u16,
    pub end: RuleEnd,
    /// `BYDAY` on a weekly rule; empty is the start's own weekday.
    pub by_day: Vec<Weekday>,
    /// Parts this subset does not expand (`BYMONTHDAY=1`), kept to report them.
    pub unexpanded: Vec<String>,
}

/// `RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=10`'s value, after the colon.
pub fn parse_rule(value: &str) -> Option<Rule> {
    let _ = value;
    todo!("M10 fill: a-calendar")
}

/// The occurrences of `event` overlapping `window`, each a [`CalEvent`] of `calendar`: the rule
/// expanded (none: the event once), `EXDATE`s removed, times resolved (`Zoned` by its zone name,
/// unknown names and `Floating` in `zone`; `Date`s stay all-day). A cancelled event has none.
/// Overrides (`recurrence_id`) are merged by the reader, not here.
pub fn occurrences(
    event: &IcsEvent,
    calendar: &CalendarName,
    window: Window,
    zone: &TimeZone,
) -> Vec<CalEvent> {
    let _ = (event, calendar, window, zone);
    todo!("M10 fill: a-calendar")
}

/// The first day of the week an occurrence of a weekly rule falls in (Monday, RFC 5545's
/// default `WKST`).
pub fn week_start(day: Date) -> Date {
    let back = i64::from(day.weekday().to_monday_zero_offset());
    day.checked_sub(jiff::Span::new().days(back)).unwrap_or(day)
}

#[cfg(test)]
mod tests {
    use jiff::civil::{Weekday, date};
    use jiff::tz::TimeZone;

    use super::{Freq, Rule, RuleEnd, occurrences, parse_rule, week_start};
    use crate::events::ics::{IcsEnd, IcsEvent, IcsStatus, IcsTime};
    use crate::events::model::{CalendarName, EventTime, Window};

    #[test]
    fn a_week_starts_on_monday() {
        assert_eq!(week_start(date(2026, 9, 26)), date(2026, 9, 21));
        assert_eq!(week_start(date(2026, 9, 21)), date(2026, 9, 21));
        assert_eq!(week_start(date(2026, 9, 27)), date(2026, 9, 21));
    }

    #[test]
    #[ignore = "M10 fill: a-calendar"]
    fn rules_parse() {
        assert_eq!(
            parse_rule("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=10"),
            Some(Rule {
                freq: Freq::Weekly,
                interval: 2,
                end: RuleEnd::Count(10),
                by_day: vec![Weekday::Monday, Weekday::Wednesday],
                unexpanded: Vec::new(),
            })
        );
        assert_eq!(
            parse_rule("FREQ=MONTHLY;BYMONTHDAY=1;UNTIL=20261231"),
            Some(Rule {
                freq: Freq::Monthly,
                interval: 1,
                end: RuleEnd::Until(IcsTime::Date(date(2026, 12, 31))),
                by_day: Vec::new(),
                unexpanded: vec!["BYMONTHDAY=1".to_owned()],
            })
        );
        assert_eq!(parse_rule("INTERVAL=2"), None);
    }

    #[test]
    #[ignore = "M10 fill: a-calendar"]
    fn a_weekly_all_day_rule_expands_inside_the_window_less_its_exdates() {
        let event = IcsEvent {
            uid: "gym".to_owned(),
            summary: "Gym".to_owned(),
            location: None,
            start: IcsTime::Date(date(2026, 9, 1)),
            end: IcsEnd::Unsaid,
            rule: Some(Rule {
                freq: Freq::Weekly,
                interval: 1,
                end: RuleEnd::Never,
                by_day: Vec::new(),
                unexpanded: Vec::new(),
            }),
            exdates: vec![IcsTime::Date(date(2026, 9, 22))],
            recurrence_id: None,
            status: IcsStatus::Confirmed,
        };
        let window = Window {
            from: date(2026, 9, 14),
            until: date(2026, 10, 1),
        };
        let days: Vec<_> = occurrences(
            &event,
            &CalendarName("std".to_owned()),
            window,
            &TimeZone::UTC,
        )
        .into_iter()
        .map(|occurrence| occurrence.when)
        .collect();
        let day = |d| EventTime::AllDay {
            first: date(2026, 9, d),
            last: date(2026, 9, d),
        };
        assert_eq!(days, vec![day(15), day(29)]);
    }
}
