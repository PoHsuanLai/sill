//! One unfolded content line split into its name, parameters and value, and the `VEVENT` being
//! read from such lines (`ics::parse_ics`'s helpers).

use super::ics::{IcsEnd, IcsEvent, IcsStatus, IcsTime};
use super::ics_value::{ValueError, parse_duration, parse_time, parse_times, unescape};
use super::rule::{Rule, parse_rule, unparsed_rule};

/// `NAME;PARAM=value;…:value`, the name and parameter names upper-cased.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ContentLine {
    pub name: String,
    pub params: Vec<(String, String)>,
    pub value: String,
}

impl ContentLine {
    /// The value of parameter `name` (upper case), unquoted.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// `text` split at its first `:` outside a quoted parameter value; `None` without one.
pub(super) fn split_line(text: &str) -> Option<ContentLine> {
    // `quoted`: inside a quoted parameter value.
    let mut quoted = false;
    let colon = text.char_indices().find_map(|(at, c)| {
        match (c, quoted) {
            ('"', _) => quoted = !quoted,
            (':', false) => return Some(at),
            _ => {}
        }
        None
    })?;
    let (head, value) = (&text[..colon], &text[colon + 1..]);
    let mut parts = split_params(head).into_iter();
    let name = parts.next()?.trim().to_ascii_uppercase();
    let params = parts
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            let value = value.trim().trim_matches('"').to_owned();
            Some((key.trim().to_ascii_uppercase(), value))
        })
        .collect();
    Some(ContentLine {
        name,
        params,
        value: value.to_owned(),
    })
}

/// `NAME;A=1;B="x;y"` at its `;`s outside quotes.
fn split_params(head: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    // `quoted`: inside a quoted parameter value.
    let mut quoted = false;
    let mut from = 0;
    for (at, c) in head.char_indices() {
        match (c, quoted) {
            ('"', _) => quoted = !quoted,
            (';', false) => {
                parts.push(&head[from..at]);
                from = at + 1;
            }
            _ => {}
        }
    }
    parts.push(&head[from..]);
    parts
}

/// A `VEVENT` being read: every property it has seen so far.
#[derive(Debug, Default)]
pub(super) struct Draft {
    uid: Option<String>,
    summary: String,
    location: Option<String>,
    start: Option<IcsTime>,
    end: Option<IcsEnd>,
    rule: Option<Rule>,
    exdates: Vec<IcsTime>,
    recurrence_id: Option<IcsTime>,
    status: IcsStatus,
}

impl Draft {
    /// Read one of the event's own properties (not a nested `VALARM`'s); why a value is bad.
    pub fn take(&mut self, content: &ContentLine) -> Result<(), ValueError> {
        match content.name.as_str() {
            "UID" => self.uid = Some(content.value.trim().to_owned()),
            "SUMMARY" => self.summary = unescape(&content.value),
            "LOCATION" => {
                self.location = Some(unescape(&content.value)).filter(|l| !l.trim().is_empty());
            }
            "DTSTART" => self.start = Some(parse_time(content)?),
            "DTEND" => self.end = Some(IcsEnd::At(parse_time(content)?)),
            "DURATION" if !matches!(self.end, Some(IcsEnd::At(_))) => {
                self.end = Some(IcsEnd::Lasts(parse_duration(&content.value)?));
            }
            "RRULE" => {
                let value = content.value.trim();
                self.rule = Some(parse_rule(value).unwrap_or_else(|| unparsed_rule(value)));
            }
            "EXDATE" => self.exdates.extend(parse_times(content)?),
            "RECURRENCE-ID" => self.recurrence_id = Some(parse_time(content)?),
            "STATUS" => self.status = status_of(&content.value),
            _ => {}
        }
        Ok(())
    }

    /// The event, when it has a `UID` and a `DTSTART`.
    pub fn finish(self) -> Option<IcsEvent> {
        Some(IcsEvent {
            uid: self.uid.filter(|uid| !uid.is_empty())?,
            summary: self.summary,
            location: self.location,
            start: self.start?,
            end: self.end.unwrap_or(IcsEnd::Unsaid),
            rule: self.rule,
            exdates: self.exdates,
            recurrence_id: self.recurrence_id,
            status: self.status,
        })
    }
}

/// `STATUS`'s value; anything unknown shows.
fn status_of(value: &str) -> IcsStatus {
    match value.trim().to_ascii_uppercase().as_str() {
        "CANCELLED" => IcsStatus::Cancelled,
        "TENTATIVE" => IcsStatus::Tentative,
        _ => IcsStatus::Confirmed,
    }
}

#[cfg(test)]
mod tests {
    use super::{ContentLine, split_line};

    #[test]
    fn lines_split_at_the_first_colon_outside_quotes() {
        let line = |name: &str, params: &[(&str, &str)], value: &str| ContentLine {
            name: name.to_owned(),
            params: params
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            value: value.to_owned(),
        };
        let cases = [
            ("UID:a:b", Some(line("UID", &[], "a:b"))),
            (
                "dtstart;tzid=\"GMT+8: Taipei\";VALUE=DATE-TIME:20260927T090000",
                Some(line(
                    "DTSTART",
                    &[("TZID", "GMT+8: Taipei"), ("VALUE", "DATE-TIME")],
                    "20260927T090000",
                )),
            ),
            ("SUMMARY:", Some(line("SUMMARY", &[], ""))),
            ("NOCOLON", None),
        ];
        for (text, want) in cases {
            assert_eq!(split_line(text), want, "{text:?}");
        }
    }
}
