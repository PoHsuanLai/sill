//! The Calendar widget (M10 lane a): the month, today marked, ISO week numbers when
//! `calendar.week_numbers` shows them, days with events marked, and previous / next month
//! (quire `IconButton{Tool}`, design/20 §1.12). [`grid::month_grid`] computes the month and
//! quire's `MonthGrid` (design/04 §39; FINDINGS "M10 freeze", Q180) draws it.

pub mod grid;
mod shown;
mod view;

pub use grid::{DayMark, DayPlace, Eventful, GridDay, GridWeek, IsoWeek, Month, MonthGrid, Step};
pub use view::CalendarWidget;

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("../../style/widget_calendar.css");
