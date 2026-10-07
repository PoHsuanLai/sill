use jiff::civil::date;
use jiff::tz::TimeZone;

use sill_shell_kit::clock::kind::DayPhase;
use sill_shell_kit::widget::clock::{ClockCity, ClockEntry};

use super::provider::{clock_time, offset_text};
use super::{ClockRow, DayThere, ZoneName, clock_entry, clock_rows, parse_zones, phase_of};

#[test]
fn zones_parse() {
    let zone = |z: &str| ZoneName(z.to_owned());
    assert_eq!(
        parse_zones(" Europe/London ;;America/New_York; Europe/London"),
        vec![zone("Europe/London"), zone("America/New_York")]
    );
    assert_eq!(parse_zones(""), Vec::new());
}

#[test]
fn offsets_read_in_hours_and_minutes() {
    let cases = [
        (0, "same time"),
        (7 * 3600, "+7h"),
        (-7 * 3600, "-7h"),
        (-(5 * 3600 + 1800), "-5h30"),
        (5 * 3600 + 2700, "+5h45"),
        (1800, "+0h30"),
    ];
    for (seconds, want) in cases {
        assert_eq!(offset_text(seconds), want, "{seconds}");
    }
}

#[test]
fn rows_say_the_time_and_the_day_there() {
    let here = TimeZone::get("Asia/Taipei").unwrap_or_else(|e| panic!("{e}"));
    let now = date(2026, 9, 26)
        .at(10, 0, 0, 0)
        .to_zoned(here.clone())
        .map(|z| z.timestamp())
        .unwrap_or_else(|e| panic!("{e}"));
    let zones = [
        ZoneName("Europe/London".to_owned()),
        ZoneName("Pacific/Auckland".to_owned()),
        ZoneName("Nowhere/Atlantis".to_owned()),
        ZoneName("Asia/Taipei".to_owned()),
    ];
    let got: Vec<_> = clock_rows(now, &zones, &here)
        .into_iter()
        .map(|r: ClockRow| (r.city, r.time.digits(), r.phase, r.day, r.offset))
        .collect();
    let row = |city: &str, time: &str, phase, offset: &str| {
        (
            city.to_owned(),
            time.to_owned(),
            phase,
            DayThere::Today,
            offset.to_owned(),
        )
    };
    assert_eq!(
        got,
        vec![
            row("London", "03:00", DayPhase::Night, "-7h"),
            row("Auckland", "14:00", DayPhase::Day, "+4h"),
            row("Taipei", "10:00", DayPhase::Day, "same time"),
        ]
    );
}

#[test]
fn a_dial_is_day_from_six_to_six() {
    let cases = [
        (0, DayPhase::Night),
        (5, DayPhase::Night),
        (6, DayPhase::Day),
        (12, DayPhase::Day),
        (17, DayPhase::Day),
        (18, DayPhase::Night),
        (23, DayPhase::Night),
    ];
    for (hour, want) in cases {
        assert_eq!(phase_of(hour), want, "{hour}");
    }
}

#[test]
fn a_row_is_a_city_with_its_offset_as_the_note() {
    let time = clock_time(9, 5);
    let row = ClockRow {
        city: "London".to_owned(),
        time,
        phase: DayPhase::Day,
        day: DayThere::Today,
        offset: "-7h".to_owned(),
    };
    assert_eq!(
        clock_entry(vec![row]),
        ClockEntry::Cities(vec![ClockCity {
            name: "London".to_owned(),
            time,
            phase: DayPhase::Day,
            notes: vec!["Today".to_owned(), "-7h".to_owned()],
        }])
    );
    assert_eq!(
        clock_entry(Vec::new()),
        ClockEntry::Absent("No cities: add them in widgets.world_clocks".to_owned())
    );
}

#[test]
fn the_day_there_is_named_across_midnight() {
    let here = TimeZone::get("Europe/London").unwrap_or_else(|e| panic!("{e}"));
    let now = date(2026, 9, 26)
        .at(23, 30, 0, 0)
        .to_zoned(here.clone())
        .map(|z| z.timestamp())
        .unwrap_or_else(|e| panic!("{e}"));
    let zones = [
        ZoneName("Asia/Kolkata".to_owned()),
        ZoneName("America/Los_Angeles".to_owned()),
        ZoneName("Pacific/Kiritimati".to_owned()),
    ];
    let got: Vec<_> = clock_rows(now, &zones, &here)
        .into_iter()
        .map(|r| (r.city, r.time.digits(), r.day, r.offset))
        .collect();
    let want = |city: &str, time: &str, day, offset: &str| {
        (city.to_owned(), time.to_owned(), day, offset.to_owned())
    };
    assert_eq!(
        got,
        vec![
            want("Kolkata", "04:00", DayThere::On(date(2026, 9, 27)), "+4h30"),
            want("Los Angeles", "15:30", DayThere::Today, "-8h"),
            want(
                "Kiritimati",
                "12:30",
                DayThere::On(date(2026, 9, 27)),
                "+13h"
            ),
        ]
    );
    let tokyo = TimeZone::get("Asia/Tokyo").unwrap_or_else(|e| panic!("{e}"));
    let rows = clock_rows(now, &[ZoneName("Europe/London".to_owned())], &tokyo);
    assert_eq!(
        rows.first().map(|r| (r.day, r.offset.as_str())),
        Some((DayThere::On(date(2026, 9, 26)), "-8h"))
    );
}

#[test]
fn the_day_line_is_today_or_the_date_there() {
    let cases = [
        (
            "same date",
            DayThere::of(date(2026, 9, 26), date(2026, 9, 26)),
            "Today",
        ),
        (
            "the next day",
            DayThere::of(date(2026, 9, 27), date(2026, 9, 26)),
            "Sun 27 Sep",
        ),
        (
            "the day before, a single-digit day",
            DayThere::of(date(2026, 10, 2), date(2026, 10, 3)),
            "Fri 2 Oct",
        ),
        (
            "across a year's end",
            DayThere::of(date(2027, 1, 1), date(2026, 12, 31)),
            "Fri 1 Jan",
        ),
    ];
    for (name, day, want) in cases {
        assert_eq!(day.text(), want, "{name}");
    }
}

#[test]
fn a_city_on_another_date_names_it_in_the_first_note() {
    let here = TimeZone::get("Europe/London").unwrap_or_else(|e| panic!("{e}"));
    let now = date(2026, 9, 26)
        .at(23, 30, 0, 0)
        .to_zoned(here.clone())
        .map(|z| z.timestamp())
        .unwrap_or_else(|e| panic!("{e}"));
    let zones = [
        ZoneName("Asia/Kolkata".to_owned()),
        ZoneName("America/Los_Angeles".to_owned()),
    ];
    let ClockEntry::Cities(cities) = clock_entry(clock_rows(now, &zones, &here)) else {
        panic!("two zones make cities");
    };
    let notes: Vec<_> = cities.into_iter().map(|c| c.notes).collect();
    assert_eq!(
        notes,
        vec![
            vec!["Sun 27 Sep".to_owned(), "+4h30".to_owned()],
            vec!["Today".to_owned(), "-8h".to_owned()],
        ]
    );
}
