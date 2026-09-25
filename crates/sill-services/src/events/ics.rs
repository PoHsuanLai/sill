//! RFC 5545, the part the widgets need, parsed by hand: content lines unfolded, `VEVENT`s with
//! their `UID`, `SUMMARY`, `LOCATION`, `DTSTART`, `DTEND` or `DURATION`, `RRULE`, `EXDATE`,
//! `STATUS`, and `RECURRENCE-ID` overrides; text values unescaped (`\n`, `\,`, `\;`, `\\`).
//! Everything else (`VTODO`, `VALARM`, `VTIMEZONE` bodies, attendees) is skipped: a `TZID` is
//! resolved by name through jiff's zone database, which knows every IANA name; a Windows-style
//! name that jiff does not know is taken as floating (the viewer's zone), not an error.

use jiff::Timestamp;
use jiff::civil::{Date, DateTime};

use super::rule::Rule;

/// A `DTSTART`, `DTEND`, `EXDATE` or `RECURRENCE-ID` value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IcsTime {
    /// `VALUE=DATE`: `20260927`.
    Date(Date),
    /// `20260927T090000Z`.
    Utc(Timestamp),
    /// `TZID=Asia/Taipei:20260927T090000`, the zone kept by name for the reader to resolve.
    Zoned { at: DateTime, zone: String },
    /// `20260927T090000`: the viewer's own zone.
    Floating(DateTime),
}

/// How long an event lasts when it has no `DTEND`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IcsEnd {
    /// `DTEND`.
    At(IcsTime),
    /// `DURATION`, in seconds (`P1DT2H` is 93600); weeks and days count as 86 400 s each.
    Lasts(i64),
    /// Neither: a date lasts one day, a time is a moment.
    Unsaid,
}

/// `STATUS:CANCELLED` events are dropped by the reader; the rest are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IcsStatus {
    #[default]
    Confirmed,
    Tentative,
    Cancelled,
}

/// One `VEVENT`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IcsEvent {
    pub uid: String,
    pub summary: String,
    pub location: Option<String>,
    pub start: IcsTime,
    pub end: IcsEnd,
    pub rule: Option<Rule>,
    pub exdates: Vec<IcsTime>,
    /// Set on an override of one occurrence of a repeating event (same `UID`).
    pub recurrence_id: Option<IcsTime>,
    pub status: IcsStatus,
}

/// Why a stream is not iCalendar.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IcsError {
    #[error("no BEGIN:VCALENDAR")]
    NotCalendar,
    /// A component opened and never closed (or closed without opening).
    #[error("line {line}: unbalanced {component}")]
    Unbalanced { line: u32, component: String },
    /// A content line without `:`, or a date or time that does not parse.
    #[error("line {line}: {reason}")]
    Bad { line: u32, reason: String },
}

/// Content lines, unfolded (a CRLF or LF followed by one space or tab continues the line), in
/// order, empty lines dropped.
pub fn unfold(text: &str) -> Vec<String> {
    let _ = text;
    todo!("M10 fill: a-calendar")
}

/// Every `VEVENT` in `text`. A `VEVENT` missing `UID` or `DTSTART` is skipped, not an error.
pub fn parse_ics(text: &str) -> Result<Vec<IcsEvent>, IcsError> {
    let _ = text;
    todo!("M10 fill: a-calendar")
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::{IcsEnd, IcsError, IcsEvent, IcsStatus, IcsTime, parse_ics, unfold};

    #[test]
    #[ignore = "M10 fill: a-calendar"]
    fn lines_unfold() {
        let cases: [(&str, &[&str]); 3] = [
            ("A:1\r\nB:2\r\n", &["A:1", "B:2"]),
            (
                "SUMMARY:long\r\n  line\r\n\tend\r\n",
                &["SUMMARY:long line end"],
            ),
            ("A:1\n\nB:2", &["A:1", "B:2"]),
        ];
        for (text, want) in cases {
            assert_eq!(unfold(text), want.to_vec(), "{text:?}");
        }
    }

    #[test]
    #[ignore = "M10 fill: a-calendar"]
    fn events_parse_with_dates_zones_and_escapes() {
        let text = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\nSUMMARY:Lunch\\, then a walk\r\n\
            LOCATION:Da'an Park\r\nDTSTART;TZID=Asia/Taipei:20260927T120000\r\nDURATION:PT1H30M\r\n\
            END:VEVENT\r\nBEGIN:VEVENT\r\nUID:b\r\nSUMMARY:Holiday\r\nDTSTART;VALUE=DATE:20261010\r\n\
            STATUS:CANCELLED\r\nEND:VEVENT\r\nBEGIN:VTODO\r\nUID:c\r\nEND:VTODO\r\nEND:VCALENDAR\r\n";
        let events = parse_ics(text).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            events,
            vec![
                IcsEvent {
                    uid: "a".to_owned(),
                    summary: "Lunch, then a walk".to_owned(),
                    location: Some("Da'an Park".to_owned()),
                    start: IcsTime::Zoned {
                        at: date(2026, 9, 27).at(12, 0, 0, 0),
                        zone: "Asia/Taipei".to_owned(),
                    },
                    end: IcsEnd::Lasts(5400),
                    rule: None,
                    exdates: Vec::new(),
                    recurrence_id: None,
                    status: IcsStatus::Confirmed,
                },
                IcsEvent {
                    uid: "b".to_owned(),
                    summary: "Holiday".to_owned(),
                    location: None,
                    start: IcsTime::Date(date(2026, 10, 10)),
                    end: IcsEnd::Unsaid,
                    rule: None,
                    exdates: Vec::new(),
                    recurrence_id: None,
                    status: IcsStatus::Cancelled,
                },
            ]
        );
    }

    #[test]
    #[ignore = "M10 fill: a-calendar"]
    fn broken_streams_are_refused() {
        assert_eq!(
            parse_ics("BEGIN:VEVENT\r\nEND:VEVENT\r\n"),
            Err(IcsError::NotCalendar)
        );
        assert!(matches!(
            parse_ics("BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:a\r\n"),
            Err(IcsError::Unbalanced { .. })
        ));
        assert!(matches!(
            parse_ics(
                "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nDTSTART:2026-09-27\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
            ),
            Err(IcsError::Bad { .. })
        ));
    }
}
