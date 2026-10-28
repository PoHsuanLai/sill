//! The words a `MonthGrid` is drawn from (design/04-COMPONENTS.md section 39). The
//! shell computes the month; these mirror its grid field for field, with the civil dates as its
//! calendar library gives them (`i16` year, `i8` month and day), so its mapping is a plain
//! `From` and quire needs no calendar of its own.

use ds::base::word::Word;
use ds::components::content::text_runs::TextLine;
use serde::{Deserialize, Serialize};

/// A calendar month: which month a grid shows. Ordered, so a change of month knows its way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MonthKey {
    /// The year.
    pub year: i16,
    /// 1-12.
    pub month: i8,
}

impl MonthKey {
    /// `2026-09`: the key the grid's weeks are mounted under.
    pub(crate) fn slug(self) -> String {
        format!("{:04}-{:02}", self.year, self.month)
    }
}

/// A civil date: which day a cell is, handed back to `onpick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DayKey {
    /// The year.
    pub year: i16,
    /// 1-12.
    pub month: i8,
    /// 1-31: also the cell's label.
    pub day: i8,
}

/// An ISO 8601 week number, 1-53: the week of the row's Thursday.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IsoWeek(pub i8);

/// Whether a cell's day is in the month shown or pads the first or last week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
pub enum DayPlace {
    /// The previous month's, before the first: drawn quieter.
    Before,
    /// The month shown.
    #[word(slug = "in")]
    InMonth,
    /// The next month's, after the last: drawn quieter.
    After,
}

/// Whether a cell is today: today sits on the accent disc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DayMark {
    /// Today: `aria-current="date"`.
    Today,
    /// Any other day.
    Plain,
}

impl DayMark {
    /// `aria-current`: written only on today.
    pub(crate) fn aria_current(self) -> Option<&'static str> {
        match self {
            DayMark::Today => Some("date"),
            DayMark::Plain => None,
        }
    }
}

/// Whether a day has an event: a busy day carries a dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
pub enum Eventful {
    /// At least one event.
    Busy,
    /// None.
    Free,
}

/// One cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonthDay {
    /// The date; its day is the label.
    pub key: DayKey,
    /// In the month shown, or a neighbour's.
    pub place: DayPlace,
    /// Today or not.
    pub mark: DayMark,
    /// Whether it has an event.
    pub events: Eventful,
}

/// One row: its ISO week and its seven days, from the first weekday.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonthWeek {
    /// The row's ISO week, drawn only with `WeekNumbers::Show`.
    pub number: IsoWeek,
    /// Seven days, the first weekday first.
    pub days: [MonthDay; 7],
}

/// A month laid out in weeks, with the words the shell formatted for it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonthGridData {
    /// Which month: the grid's identity, and the order a change slides by.
    pub month: MonthKey,
    /// The header's words: "September 2026".
    pub title: TextLine,
    /// The column heads, starting at the first weekday: weekday initials.
    pub heads: [TextLine; 7],
    /// As many rows as the month touches (four to six), padded with the neighbours' days.
    pub weeks: Vec<MonthWeek>,
}

/// Whether the grid leads each row with its ISO week (`calendar.week_numbers`, design/22
/// section 3.21).
///
/// A word of the widget contract (a month widget's face carries it), so it stays this crate's
/// own: the setting's `sill_settings::WeekNumbers` maps onto it in the calendar widget, since
/// this kit cannot name the settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
pub enum WeekNumbers {
    /// Seven columns only.
    #[default]
    Hide,
    /// A quieter column of week numbers first.
    Show,
}

/// Which way the header's buttons step the month.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonthStep {
    /// The month before.
    Previous,
    /// The month after.
    Next,
}

impl MonthStep {
    /// The button's name.
    pub(crate) fn label(self) -> &'static str {
        match self {
            MonthStep::Previous => "Previous month",
            MonthStep::Next => "Next month",
        }
    }
}
