//! RFC 5545, the part the widgets need, parsed by hand: content lines unfolded, `VEVENT`s with
//! their `UID`, `SUMMARY`, `LOCATION`, `DTSTART`, `DTEND` or `DURATION`, `RRULE`, `EXDATE`,
//! `STATUS`, and `RECURRENCE-ID` overrides; text values unescaped (`\n`, `\,`, `\;`, `\\`).
//! Everything else (`VTODO`, `VALARM`, `VTIMEZONE` bodies, attendees) is skipped: a `TZID` is
//! resolved by name through jiff's zone database, which knows every IANA name; a Windows-style
//! name that jiff does not know is taken as floating (the viewer's zone), not an error.

use jiff::Timestamp;
use jiff::civil::{Date, DateTime};

use super::ics_line::{Draft, split_line};
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
///
/// A continuation starting with a space loses that space (RFC 5545 §3.1); one starting with a
/// tab keeps one space in its place, since the producers that fold with a tab (Outlook, Lotus)
/// fold at a word gap and drop the space they folded at.
pub fn unfold(text: &str) -> Vec<String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        match (raw.chars().next(), lines.last_mut()) {
            (Some(' '), Some(last)) => last.push_str(&raw[1..]),
            (Some('\t'), Some(last)) => {
                last.push(' ');
                last.push_str(&raw[1..]);
            }
            _ if raw.trim().is_empty() => {}
            _ => lines.push(raw.to_owned()),
        }
    }
    lines
}

/// Every `VEVENT` in `text`. A `VEVENT` missing `UID` or `DTSTART` is skipped, not an error.
/// Several `VCALENDAR`s one after another (a concatenated export) are read in turn.
pub fn parse_ics(text: &str) -> Result<Vec<IcsEvent>, IcsError> {
    let lines = unfold(text);
    let opens_calendar = lines
        .first()
        .is_some_and(|first| first.trim().eq_ignore_ascii_case("BEGIN:VCALENDAR"));
    if !opens_calendar {
        return Err(IcsError::NotCalendar);
    }
    let mut stack: Vec<String> = Vec::new();
    let mut open: Option<Draft> = None;
    let mut events = Vec::new();
    for (index, text) in lines.iter().enumerate() {
        let line = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let content = split_line(text).ok_or_else(|| IcsError::Bad {
            line,
            reason: format!("no ':' in {:?}", text.chars().take(40).collect::<String>()),
        })?;
        match content.name.as_str() {
            "BEGIN" => {
                let component = content.value.trim().to_ascii_uppercase();
                if component == "VEVENT" && stack.len() == 1 {
                    open = Some(Draft::default());
                }
                stack.push(component);
            }
            "END" => {
                let component = content.value.trim().to_ascii_uppercase();
                if stack.last() != Some(&component) {
                    return Err(IcsError::Unbalanced { line, component });
                }
                stack.pop();
                if component == "VEVENT" && stack.len() == 1 {
                    events.extend(open.take().and_then(Draft::finish));
                }
            }
            _ if stack.len() == 2 => {
                if let Some(draft) = open.as_mut() {
                    draft
                        .take(&content)
                        .map_err(|reason| IcsError::Bad { line, reason })?;
                }
            }
            _ => {}
        }
    }
    match stack.pop() {
        Some(component) => Err(IcsError::Unbalanced {
            line: u32::try_from(lines.len()).unwrap_or(u32::MAX),
            component,
        }),
        None => Ok(events),
    }
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;

    use super::{IcsEnd, IcsError, IcsEvent, IcsStatus, IcsTime, parse_ics, unfold};

    #[test]
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
