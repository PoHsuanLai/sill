//! `RRULE`, the common subset, and turning a parsed event into the occurrences a window holds.
//! Expanded: `FREQ` daily to yearly, `INTERVAL`, `COUNT`, `UNTIL`, `BYDAY` (weekdays; with an
//! ordinal on a monthly rule, or a yearly one with `BYMONTH`), `BYMONTHDAY`, `BYMONTH` and
//! `WKST`. `BYSETPOS`, `BYYEARDAY`, `BYWEEKNO`, `BYHOUR` and the like are not: an event whose
//! rule uses them shows its first occurrence only, and the reader logs it once.

use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;

use super::ics::{IcsEvent, IcsStatus, IcsTime};
use super::occur::{Exclusions, Shape, overlaps};
use super::rule_expand::{expand, plan};
use super::{CalEvent, CalendarName, EventUid, Window};

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
    /// Parts outside the fields above (`BYMONTHDAY=1`), as written: [`occurrences`] expands the
    /// ones it knows (`BYMONTHDAY`, `BYMONTH`, an ordinal `BYDAY`, `WKST`) and shows an event
    /// with any other once, which the reader reports.
    pub unexpanded: Vec<String>,
}

/// `RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=10`'s value, after the colon.
///
/// `None` without a known `FREQ` or with a number that does not read. A `FREQ` finer than a day
/// (`HOURLY`) reads as daily with `FREQ=HOURLY` unexpanded, so the event shows once. A `BYDAY`
/// with an ordinal (`2TU`, `-1FR`) stays whole in `unexpanded`, where [`occurrences`] reads it
/// for a monthly or yearly rule, as it reads `BYMONTHDAY`, `BYMONTH` and `WKST`.
pub fn parse_rule(value: &str) -> Option<Rule> {
    let mut freq = None;
    let mut rule = Rule {
        freq: Freq::Daily,
        interval: 1,
        end: RuleEnd::Never,
        by_day: Vec::new(),
        unexpanded: Vec::new(),
    };
    for part in value.split(';').map(str::trim).filter(|p| !p.is_empty()) {
        let (key, val) = part.split_once('=')?;
        let key = key.trim().to_ascii_uppercase();
        let val = val.trim();
        match key.as_str() {
            "FREQ" => {
                let (read, finer) = freq_of(val)?;
                freq = Some(read);
                rule.unexpanded.extend(finer.map(|f| format!("FREQ={f}")));
            }
            "INTERVAL" => rule.interval = val.parse().ok().filter(|n| *n > 0)?,
            "COUNT" => rule.end = RuleEnd::Count(val.parse().ok()?),
            "UNTIL" => rule.end = RuleEnd::Until(until_of(val)?),
            "BYDAY" => match plain_days(val) {
                Some(days) => rule.by_day = days,
                None => rule.unexpanded.push(format!("BYDAY={val}")),
            },
            _ => rule.unexpanded.push(format!("{key}={val}")),
        }
    }
    rule.freq = freq?;
    Some(rule)
}

/// A rule that could not be read, kept so the event shows once and the reader can say why.
pub(super) fn unparsed_rule(value: &str) -> Rule {
    Rule {
        freq: Freq::Daily,
        interval: 1,
        end: RuleEnd::Count(1),
        by_day: Vec::new(),
        unexpanded: vec![format!("RRULE={value}")],
    }
}

/// `FREQ`'s value, and the finer frequency this subset reads as daily.
fn freq_of(value: &str) -> Option<(Freq, Option<&'static str>)> {
    match value.to_ascii_uppercase().as_str() {
        "DAILY" => Some((Freq::Daily, None)),
        "WEEKLY" => Some((Freq::Weekly, None)),
        "MONTHLY" => Some((Freq::Monthly, None)),
        "YEARLY" => Some((Freq::Yearly, None)),
        "HOURLY" => Some((Freq::Daily, Some("HOURLY"))),
        "MINUTELY" => Some((Freq::Daily, Some("MINUTELY"))),
        "SECONDLY" => Some((Freq::Daily, Some("SECONDLY"))),
        _ => None,
    }
}

/// `UNTIL`: a date, a UTC date-time, or (against the RFC, but written) a floating one.
fn until_of(value: &str) -> Option<IcsTime> {
    let line = super::ics_line::ContentLine {
        name: "UNTIL".to_owned(),
        params: Vec::new(),
        value: value.to_owned(),
    };
    super::ics_value::parse_time(&line).ok()
}

/// `MO,WE` as weekdays; `None` when any entry has an ordinal or is not a weekday.
fn plain_days(value: &str) -> Option<Vec<Weekday>> {
    value.split(',').map(|day| weekday_of(day.trim())).collect()
}

/// `MO` to `SU`.
pub(super) fn weekday_of(code: &str) -> Option<Weekday> {
    match code.to_ascii_uppercase().as_str() {
        "MO" => Some(Weekday::Monday),
        "TU" => Some(Weekday::Tuesday),
        "WE" => Some(Weekday::Wednesday),
        "TH" => Some(Weekday::Thursday),
        "FR" => Some(Weekday::Friday),
        "SA" => Some(Weekday::Saturday),
        "SU" => Some(Weekday::Sunday),
        _ => None,
    }
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
    if event.status == IcsStatus::Cancelled {
        return Vec::new();
    }
    let shape = Shape::of(event, zone);
    let bounds = shape.bounds(window);
    let days = match event.rule.as_ref().map(plan) {
        Some(Ok(pattern)) => expand(&pattern, shape.first_day(), bounds, |day| {
            shape.past_until(day, &pattern.end, zone)
        }),
        Some(Err(_)) | None => vec![shape.first_day()],
    };
    let skip = Exclusions::of(&event.exdates, zone);
    days.into_iter()
        .filter_map(|day| shape.at(day))
        .filter(|when| !skip.covers(when, zone))
        .filter(|when| overlaps(when, window, zone))
        .map(|when| CalEvent {
            uid: EventUid(event.uid.clone()),
            title: event.summary.clone(),
            when,
            location: event.location.clone(),
            calendar: calendar.clone(),
        })
        .collect()
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
    use crate::events::{CalendarName, EventTime, Window};
    use crate::events::{IcsEnd, IcsEvent, IcsStatus, IcsTime};

    #[test]
    fn a_week_starts_on_monday() {
        assert_eq!(week_start(date(2026, 9, 26)), date(2026, 9, 21));
        assert_eq!(week_start(date(2026, 9, 21)), date(2026, 9, 21));
        assert_eq!(week_start(date(2026, 9, 27)), date(2026, 9, 21));
    }

    #[test]
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
