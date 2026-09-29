//! The Calendar widget's provider: the month, today marked, ISO week numbers when
//! `calendar.week_numbers` shows them, days with events marked, today's weekday and date, and
//! today's events still to come. [`grid::month_grid`] computes the month and quire's
//! `MonthWidget` draws it on its card (Small the month on the desktop, Medium today beside it in
//! the notification center); its step buttons send `MonthIntent::Step`, which shifts the month.

pub mod grid;
mod io;
mod shown;

pub use grid::{DayMark, DayPlace, Eventful, GridDay, GridWeek, IsoWeek, Month, MonthGrid, Step};
pub use io::CalendarProvider;
