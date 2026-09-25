use jiff::civil::{Date, date};

use super::{Bounds, Until, expand, plan, unsupported_parts};
use crate::events::rule::parse_rule;

fn bounds(from: Date, to: Date) -> Bounds {
    Bounds {
        keep_from: from,
        stop_after: to,
    }
}

/// `rule` from `first` over 2026 (and a little of 2027), `UNTIL` left to the rule's `COUNT`.
fn days(rule: &str, first: Date) -> Vec<Date> {
    let rule = parse_rule(rule).unwrap_or_else(|| panic!("{rule}"));
    let pattern = plan(&rule).unwrap_or_else(|parts| panic!("{parts:?}"));
    expand(
        &pattern,
        first,
        bounds(date(2026, 1, 1), date(2027, 3, 1)),
        |_| Until::Before,
    )
}

#[test]
fn rules_expand_to_their_days() {
    let d = |m, d| date(2026, m, d);
    let cases: [(&str, Date, Vec<Date>); 10] = [
        (
            "FREQ=DAILY;COUNT=3",
            d(9, 29),
            vec![d(9, 29), d(9, 30), d(10, 1)],
        ),
        (
            "FREQ=DAILY;INTERVAL=10;COUNT=3",
            d(9, 1),
            vec![d(9, 1), d(9, 11), d(9, 21)],
        ),
        (
            "FREQ=DAILY;BYDAY=MO,TU,WE,TH,FR;COUNT=4",
            d(9, 25),
            vec![d(9, 25), d(9, 28), d(9, 29), d(9, 30)],
        ),
        (
            "FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE;COUNT=5",
            d(9, 7),
            vec![d(9, 7), d(9, 9), d(9, 21), d(9, 23), d(10, 5)],
        ),
        (
            "FREQ=WEEKLY;WKST=SU;INTERVAL=2;BYDAY=SU,SA;COUNT=4",
            d(9, 5),
            vec![d(9, 5), d(9, 13), d(9, 19), d(9, 27)],
        ),
        (
            "FREQ=MONTHLY;COUNT=4",
            d(10, 31),
            vec![d(10, 31), d(12, 31), date(2027, 1, 31)],
        ),
        (
            "FREQ=MONTHLY;BYMONTHDAY=1,-1;COUNT=4",
            d(9, 1),
            vec![d(9, 1), d(9, 30), d(10, 1), d(10, 31)],
        ),
        (
            "FREQ=MONTHLY;BYDAY=-1FR;COUNT=3",
            d(9, 25),
            vec![d(9, 25), d(10, 30), d(11, 27)],
        ),
        (
            "FREQ=YEARLY;BYMONTH=11;BYDAY=4TH;COUNT=2",
            d(11, 26),
            vec![d(11, 26)],
        ),
        (
            "FREQ=MONTHLY;BYMONTHDAY=13;BYDAY=FR;COUNT=2",
            d(2, 13),
            vec![d(2, 13), d(3, 13)],
        ),
    ];
    for (rule, first, want) in cases {
        assert_eq!(days(rule, first), want, "{rule} from {first}");
    }
}

#[test]
fn a_leap_day_repeats_only_in_leap_years() {
    let got = days("FREQ=YEARLY", date(2024, 2, 29));
    assert_eq!(got, Vec::<Date>::new(), "2025 to 2027 have no 29 February");
    let rule = parse_rule("FREQ=YEARLY").unwrap_or_else(|| panic!("rule"));
    let pattern = plan(&rule).unwrap_or_else(|parts| panic!("{parts:?}"));
    let got = expand(
        &pattern,
        date(2024, 2, 29),
        bounds(date(2024, 1, 1), date(2029, 1, 1)),
        |_| Until::Before,
    );
    assert_eq!(got, vec![date(2024, 2, 29), date(2028, 2, 29)]);
}

#[test]
fn until_and_count_stop_and_early_days_still_count() {
    let rule = parse_rule("FREQ=DAILY;COUNT=5").unwrap_or_else(|| panic!("rule"));
    let pattern = plan(&rule).unwrap_or_else(|parts| panic!("{parts:?}"));
    let got = expand(
        &pattern,
        date(2026, 9, 1),
        bounds(date(2026, 9, 4), date(2026, 12, 1)),
        |_| Until::Before,
    );
    assert_eq!(got, vec![date(2026, 9, 4), date(2026, 9, 5)]);
    let rule = parse_rule("FREQ=DAILY").unwrap_or_else(|| panic!("rule"));
    let pattern = plan(&rule).unwrap_or_else(|parts| panic!("{parts:?}"));
    let got = expand(
        &pattern,
        date(2026, 9, 1),
        bounds(date(2026, 9, 1), date(2026, 12, 1)),
        |day| match day > date(2026, 9, 3) {
            true => Until::Past,
            false => Until::Before,
        },
    );
    assert_eq!(
        got,
        vec![date(2026, 9, 1), date(2026, 9, 2), date(2026, 9, 3)]
    );
}

#[test]
fn what_is_not_expanded_is_named() {
    let cases = [
        (
            "FREQ=MONTHLY;BYDAY=MO,TU,WE,TH,FR;BYSETPOS=-1",
            vec!["BYSETPOS=-1"],
        ),
        ("FREQ=HOURLY;INTERVAL=2", vec!["FREQ=HOURLY"]),
        ("FREQ=YEARLY;BYDAY=20MO", vec!["BYDAY=20MO"]),
        (
            "FREQ=WEEKLY;BYMONTHDAY=3",
            vec!["Weekly with this BYDAY/BYMONTHDAY"],
        ),
        (
            "FREQ=YEARLY;BYDAY=1MO",
            vec!["Yearly with this BYDAY/BYMONTHDAY"],
        ),
        ("FREQ=MONTHLY;BYMONTHDAY=1", vec![]),
        ("FREQ=WEEKLY;WKST=SU;BYDAY=TU,TH", vec![]),
    ];
    for (text, want) in cases {
        let rule = parse_rule(text).unwrap_or_else(|| panic!("{text}"));
        assert_eq!(unsupported_parts(&rule), want, "{text}");
    }
}
