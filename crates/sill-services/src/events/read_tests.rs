//! The reader over realistic files: KOrganizer's `std.ics`, a Google Calendar export and a
//! vdirsyncer folder (`tests/fixtures/calendars`), in Taipei, September and October 2026.

use std::path::PathBuf;

use jiff::civil::{Date, date};
use jiff::tz::TimeZone;

use super::{calendars_in, occurrences_of, read_places};
use crate::events::model::{CalEvent, CalendarName, EventTime, Window};
use crate::events::sources::IcsPlace;

const KORGANIZER: &str = include_str!("../../tests/fixtures/calendars/korganizer-std.ics");
const GOOGLE: &str = include_str!("../../tests/fixtures/calendars/google-export.ics");

fn taipei() -> TimeZone {
    TimeZone::get("Asia/Taipei").unwrap_or_else(|e| panic!("{e}"))
}

fn autumn() -> Window {
    Window {
        from: date(2026, 9, 1),
        until: date(2026, 11, 1),
    }
}

fn read(text: &str) -> Vec<CalEvent> {
    occurrences_of(text, "cal", autumn(), &taipei()).unwrap_or_else(|e| panic!("{e}"))
}

/// The local days `title`'s occurrences start on, in order.
fn days_of(events: &[CalEvent], title: &str) -> Vec<Date> {
    let mut days: Vec<Date> = events
        .iter()
        .filter(|event| event.title == title)
        .map(|event| match event.when {
            EventTime::AllDay { first, .. } => first,
            EventTime::Timed { start, .. } => start.to_zoned(taipei()).date(),
        })
        .collect();
    days.sort();
    days
}

/// `title`'s first occurrence's local start and end, as `HH:MM`.
fn clock_of(events: &[CalEvent], title: &str) -> (String, String) {
    let event = events
        .iter()
        .find(|event| event.title == title)
        .unwrap_or_else(|| panic!("no {title}"));
    let hm = |at: jiff::Timestamp| at.to_zoned(taipei()).strftime("%H:%M").to_string();
    match event.when {
        EventTime::Timed { start, end } => (hm(start), hm(end)),
        EventTime::AllDay { .. } => panic!("{title} is all day"),
    }
}

fn september(days: &[i8]) -> Vec<Date> {
    days.iter().map(|d| date(2026, 9, *d)).collect()
}

fn october(days: &[i8]) -> Vec<Date> {
    days.iter().map(|d| date(2026, 10, *d)).collect()
}

#[test]
fn korganizers_calendar_expands() {
    let events = read(KORGANIZER);
    let standup = days_of(&events, "Team standup");
    let mut want = september(&[2, 4, 7, 9, 11, 14, 18, 21, 23, 25, 28, 30]);
    want.extend(october(&[2, 5, 7, 9, 12, 14, 16, 19, 21, 23, 26, 28, 30]));
    assert_eq!(standup, want, "Mon, Wed, Fri less the 16th, until the 31st");
    assert_eq!(
        clock_of(&events, "Team standup"),
        ("09:30".into(), "09:45".into())
    );
    assert_eq!(days_of(&events, "Mum's birthday"), september(&[28]));
    assert_eq!(
        days_of(&events, "Rent"),
        vec![date(2026, 9, 1), date(2026, 10, 1)]
    );
    let ski = events
        .iter()
        .find(|e| e.title == "Ski trip")
        .map(|e| e.when);
    assert_eq!(
        ski,
        Some(EventTime::AllDay {
            first: date(2026, 10, 9),
            last: date(2026, 10, 11),
        })
    );
    let dentist = events.iter().find(|e| e.title == "Dentist");
    assert_eq!(
        dentist.and_then(|e| e.location.clone()).as_deref(),
        Some("Da'an Dental, 2F; ask for Dr. Lin")
    );
    assert!(days_of(&events, "Dinner at Tim's").is_empty(), "cancelled");
    assert!(days_of(&events, "File taxes").is_empty(), "a VTODO");
    assert_eq!(events.len(), 25 + 1 + 2 + 1 + 1);
}

#[test]
fn a_google_export_expands_with_its_overrides() {
    let events = read(GOOGLE);
    // Tue and Thu 10:00 in New York is 22:00 in Taipei; the 24th moved to the 25th and the
    // 1st cancelled.
    let mut want = september(&[15, 17, 22, 29]);
    want.extend(october(&[6, 8, 13, 15, 20, 22, 27, 29]));
    assert_eq!(days_of(&events, "NY sync"), want);
    assert_eq!(
        clock_of(&events, "NY sync"),
        ("22:00".into(), "23:00".into())
    );
    assert_eq!(
        days_of(&events, "NY sync (moved to Friday)"),
        september(&[25])
    );
    assert_eq!(
        days_of(&events, "Book club"),
        vec![date(2026, 9, 8), date(2026, 10, 13)],
        "the second Tuesday"
    );
    assert_eq!(
        clock_of(&events, "Book club"),
        ("10:00".into(), "11:00".into())
    );
    assert_eq!(
        days_of(&events, "Payday"),
        september(&[30]),
        "BYSETPOS: once"
    );
    assert_eq!(days_of(&events, "Morning run"), october(&[5, 6, 7, 8, 9]));
    assert_eq!(days_of(&events, "Team offsite"), october(&[6]));
}

#[test]
fn a_vdirsyncer_folder_is_one_calendar_per_collection() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/calendars/vdir");
    let calendars = calendars_in(&IcsPlace::Dir(root.clone())).unwrap_or_else(|e| panic!("{e}"));
    let names: Vec<_> = calendars.iter().map(|c| c.name.0.as_str()).collect();
    assert_eq!(names, ["personal", "work"]);
    assert!(
        calendars.iter().all(|c| c.files.len() == 2),
        "{calendars:?}"
    );

    let read = read_places(&[IcsPlace::Dir(root)], autumn(), &taipei());
    assert_eq!(read.calendars, 2);
    assert_eq!(read.problems, Vec::new());
    let events = read.events;
    assert_eq!(days_of(&events, "Coffee with Mei"), september(&[27]));
    assert_eq!(
        clock_of(&events, "Coffee with Mei"),
        ("15:00".into(), "16:00".into()),
        "floating: the viewer's own clock"
    );
    assert_eq!(days_of(&events, "Yoga"), september(&[3, 10, 17, 24]));
    let review = "Quarterly review with the platform team";
    assert_eq!(days_of(&events, review), october(&[1]), "tab-folded");
    assert_eq!(
        clock_of(&events, review),
        ("10:00".into(), "11:30".into()),
        "a Windows zone name is floating"
    );
    let on_call: Vec<_> = days_of(&events, "On call");
    assert_eq!(
        on_call,
        vec![date(2026, 9, 28), date(2026, 10, 12), date(2026, 10, 26)]
    );
    assert_eq!(
        clock_of(&events, "On call"),
        ("09:00".into(), "09:00".into()),
        "a day long, from a Mozilla-prefixed zone"
    );
    let work = events
        .iter()
        .find(|e| e.title == "On call")
        .map(|e| e.calendar.clone());
    assert_eq!(work, Some(CalendarName("work".to_owned())));
}

#[test]
fn a_missing_place_is_a_problem_and_costs_nothing_else() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/calendars");
    let read = read_places(
        &[
            IcsPlace::File(root.join("nowhere.ics")),
            IcsPlace::File(root.join("korganizer-std.ics")),
        ],
        autumn(),
        &taipei(),
    );
    assert_eq!(read.calendars, 1);
    assert_eq!(read.problems.len(), 1);
    assert_eq!(read.events.len(), 30);
}
