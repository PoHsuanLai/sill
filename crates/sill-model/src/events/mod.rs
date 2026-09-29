//! Calendar events: the agenda, iCalendar parsed, recurrence expanded, and the sources a setting
//! names.

mod agenda;
mod command;
mod ics;
mod ics_line;
mod ics_value;
mod model;
mod occur;
mod rule;
mod rule_expand;
mod sources;
mod step;
#[cfg(test)]
mod tests;

pub use agenda::{Days, up_next};
pub use command::{AgendaEvent, EventsCommand};
pub use ics::{IcsEnd, IcsError, IcsEvent, IcsStatus, IcsTime, parse_ics, unfold};
pub use model::{
    Agenda, CalEvent, CalendarName, EventTime, EventUid, EventsError, SourceStatus, Window,
    held_window,
};
pub use rule::{Freq, Rule, RuleEnd, occurrences, parse_rule, week_start};
pub use rule_expand::unsupported_parts;
pub use sources::{AutoEnv, EventSources, IcsPlace, auto_places, parse_sources};
pub use step::step;
