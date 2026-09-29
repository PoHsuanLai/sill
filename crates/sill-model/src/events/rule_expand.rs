//! A [`Rule`] expanded into the days its occurrences fall on: which parts this subset knows
//! ([`plan`]), and the days period by period ([`expand`]), civil and zone-free (the caller
//! puts the time of day and the zone back on).

use jiff::ToSpan;
use jiff::civil::{Date, Weekday};

use super::rule::{Freq, Rule, RuleEnd, weekday_of};

/// A rule this subset can expand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Pattern {
    pub freq: Freq,
    pub interval: i32,
    pub end: RuleEnd,
    /// `BYDAY`: a weekday, with its ordinal in the month when it has one.
    pub days: Vec<(Option<i8>, Weekday)>,
    /// `BYMONTHDAY`: 1 to 31, or -1 (the last) to -31.
    pub month_days: Vec<i8>,
    /// `BYMONTH`: 1 to 12.
    pub months: Vec<i8>,
    /// `WKST`, Monday when unsaid.
    pub week_start: Weekday,
}

/// The days kept and where to stop: occurrences before `keep_from` are counted (for `COUNT`)
/// but dropped; the expansion ends after `stop_after`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Bounds {
    pub keep_from: Date,
    pub stop_after: Date,
}

/// Whether an occurrence is past the rule's `UNTIL`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Until {
    Before,
    Past,
}

/// The most periods one expansion walks: a daily rule over 270 years.
const MOST_PERIODS: i32 = 100_000;

/// `rule` as a pattern, or the parts this subset does not expand.
pub(super) fn plan(rule: &Rule) -> Result<Pattern, Vec<String>> {
    let mut pattern = Pattern {
        freq: rule.freq,
        interval: i32::from(rule.interval.max(1)),
        end: rule.end.clone(),
        days: rule.by_day.iter().map(|day| (None, *day)).collect(),
        month_days: Vec::new(),
        months: Vec::new(),
        week_start: Weekday::Monday,
    };
    let mut unknown = Vec::new();
    for part in &rule.unexpanded {
        if read_part(part, &mut pattern).is_none() {
            unknown.push(part.clone());
        }
    }
    unknown.extend(unfit(&pattern));
    match unknown.is_empty() {
        true => Ok(pattern),
        false => Err(unknown),
    }
}

/// The parts of `rule` an expansion leaves out (the event then shows once).
pub fn unsupported_parts(rule: &Rule) -> Vec<String> {
    plan(rule).err().unwrap_or_default()
}

/// Read one `KEY=VALUE` this subset knows into `pattern`.
fn read_part(part: &str, pattern: &mut Pattern) -> Option<()> {
    let (key, value) = part.split_once('=')?;
    let list = || value.split(',').map(str::trim);
    match key {
        "BYMONTHDAY" => {
            pattern.month_days = list()
                .map(|n| n.parse::<i8>().ok().filter(|n| (1..=31).contains(&n.abs())))
                .collect::<Option<_>>()?;
        }
        "BYMONTH" => {
            pattern.months = list()
                .map(|n| n.parse::<i8>().ok().filter(|n| (1..=12).contains(n)))
                .collect::<Option<_>>()?;
        }
        "BYDAY" => pattern.days = list().map(ordinal_day).collect::<Option<_>>()?,
        "WKST" => pattern.week_start = weekday_of(value)?,
        _ => return None,
    }
    Some(())
}

/// `2TU`, `-1FR`, `+3MO` or a plain `WE`.
fn ordinal_day(text: &str) -> Option<(Option<i8>, Weekday)> {
    let split = text.len().checked_sub(2)?;
    let (number, code) = text.split_at(split);
    let day = weekday_of(code)?;
    match number {
        "" => Some((None, day)),
        n => {
            let n: i8 = n.trim_start_matches('+').parse().ok()?;
            ((1..=5).contains(&n.abs())).then_some((Some(n), day))
        }
    }
}

/// The combinations this subset does not expand, named.
fn unfit(pattern: &Pattern) -> Vec<String> {
    let ordinal = pattern.days.iter().any(|(n, _)| n.is_some());
    let plain = pattern.days.iter().any(|(n, _)| n.is_none());
    let by_month = !pattern.months.is_empty();
    let month_days = !pattern.month_days.is_empty();
    let fits = match pattern.freq {
        Freq::Daily | Freq::Weekly => !ordinal && !(month_days && pattern.freq == Freq::Weekly),
        Freq::Monthly => !(ordinal && month_days),
        Freq::Yearly => (by_month || (!ordinal && !plain)) && !(ordinal && month_days),
    };
    match fits {
        true => Vec::new(),
        false => vec![format!("{:?} with this BYDAY/BYMONTHDAY", pattern.freq)],
    }
}

/// Every day `pattern` puts an occurrence on, starting at `first` (always the first, as
/// `DTSTART` is), within `bounds`, until `past` says the rule's `UNTIL` is passed or its
/// `COUNT` is spent.
pub(super) fn expand(
    pattern: &Pattern,
    first: Date,
    bounds: Bounds,
    past: impl Fn(Date) -> Until,
) -> Vec<Date> {
    let limit = match pattern.end {
        RuleEnd::Count(n) => n,
        RuleEnd::Never | RuleEnd::Until(_) => u32::MAX,
    };
    let keep = |day: Date| day >= bounds.keep_from && day <= bounds.stop_after;
    let mut days: Vec<Date> = [first].into_iter().filter(|d| keep(*d)).collect();
    let mut made: u32 = 1;
    for k in 0..MOST_PERIODS {
        let Some((start, candidates)) = period(pattern, first, k) else {
            break;
        };
        if start > bounds.stop_after {
            break;
        }
        for day in candidates.into_iter().filter(|day| *day > first) {
            if made >= limit || day > bounds.stop_after || past(day) == Until::Past {
                return days;
            }
            made += 1;
            if keep(day) {
                days.push(day);
            }
        }
    }
    days
}

/// Period `k`'s first day and its candidate days, in order.
fn period(pattern: &Pattern, first: Date, k: i32) -> Option<(Date, Vec<Date>)> {
    let step = k.checked_mul(pattern.interval)?;
    let in_months = |day: &Date| pattern.months.is_empty() || pattern.months.contains(&day.month());
    match pattern.freq {
        Freq::Daily => {
            let day = first.checked_add(i64::from(step).days()).ok()?;
            let fits = in_months(&day) && daily_fits(pattern, day);
            Some((day, [day].into_iter().filter(|_| fits).collect()))
        }
        Freq::Weekly => {
            let back = i64::from(first.weekday().since(pattern.week_start));
            let start = first.checked_sub(back.days()).ok()?;
            let start = start.checked_add((i64::from(step) * 7).days()).ok()?;
            let wanted = |day: Date| match pattern.days.is_empty() {
                true => day.weekday() == first.weekday(),
                false => pattern.days.iter().any(|(_, w)| *w == day.weekday()),
            };
            let days = (0..7)
                .filter_map(|i| start.checked_add(i.days()).ok())
                .filter(|day| wanted(*day) && in_months(day))
                .collect();
            Some((start, days))
        }
        Freq::Monthly => {
            let total = i32::from(first.year()) * 12 + i32::from(first.month() - 1) + step;
            let year = i16::try_from(total.div_euclid(12)).ok()?;
            let month = i8::try_from(total.rem_euclid(12) + 1).ok()?;
            let start = Date::new(year, month, 1).ok()?;
            let days = month_days(pattern, first, year, month)
                .into_iter()
                .filter(in_months)
                .collect();
            Some((start, days))
        }
        Freq::Yearly => {
            let year = i16::try_from(i32::from(first.year()) + step).ok()?;
            let start = Date::new(year, 1, 1).ok()?;
            let mut months = match pattern.months.is_empty() {
                true => vec![first.month()],
                false => pattern.months.clone(),
            };
            months.sort_unstable();
            months.dedup();
            let days = months
                .into_iter()
                .flat_map(|month| month_days(pattern, first, year, month))
                .collect();
            Some((start, days))
        }
    }
}

/// A daily rule's `BYDAY` and `BYMONTHDAY` as filters.
fn daily_fits(pattern: &Pattern, day: Date) -> bool {
    let weekday = pattern.days.is_empty() || pattern.days.iter().any(|(_, w)| *w == day.weekday());
    let month_day = pattern.month_days.is_empty()
        || pattern
            .month_days
            .iter()
            .any(|n| resolve_month_day(day.year(), day.month(), *n) == Some(day));
    weekday && month_day
}

/// The days of one month a monthly or yearly rule names, in order: its `BYMONTHDAY`s (kept to
/// its plain `BYDAY`s when it has both), else its `BYDAY`s, else the first day's own day.
fn month_days(pattern: &Pattern, first: Date, year: i16, month: i8) -> Vec<Date> {
    let mut days: Vec<Date> = match (pattern.month_days.is_empty(), pattern.days.is_empty()) {
        (false, _) => pattern
            .month_days
            .iter()
            .filter_map(|n| resolve_month_day(year, month, *n))
            .filter(|day| {
                pattern.days.is_empty() || pattern.days.iter().any(|(_, w)| *w == day.weekday())
            })
            .collect(),
        (true, false) => pattern
            .days
            .iter()
            .flat_map(|(n, weekday)| weekdays_in(year, month, *n, *weekday))
            .collect(),
        (true, true) => Date::new(year, month, first.day())
            .ok()
            .into_iter()
            .collect(),
    };
    days.sort_unstable();
    days.dedup();
    days
}

/// Day `n` of the month (negative from its end), when the month has it.
fn resolve_month_day(year: i16, month: i8, n: i8) -> Option<Date> {
    let first = Date::new(year, month, 1).ok()?;
    let day = match n > 0 {
        true => n,
        false => first.days_in_month() + 1 + n,
    };
    Date::new(year, month, day).ok()
}

/// Every `weekday` of the month, or only the `n`th (negative from the end).
fn weekdays_in(year: i16, month: i8, n: Option<i8>, weekday: Weekday) -> Vec<Date> {
    let Ok(first) = Date::new(year, month, 1) else {
        return Vec::new();
    };
    let all: Vec<Date> = (0..first.days_in_month())
        .filter_map(|i| first.checked_add(i64::from(i).days()).ok())
        .filter(|day| day.weekday() == weekday)
        .collect();
    match n {
        None => all,
        Some(n) if n > 0 => all
            .get(usize::try_from(n - 1).unwrap_or(usize::MAX))
            .copied()
            .into_iter()
            .collect(),
        Some(n) => all
            .len()
            .checked_sub(usize::from(n.unsigned_abs()))
            .and_then(|i| all.get(i).copied())
            .into_iter()
            .collect(),
    }
}

#[cfg(test)]
mod tests;
