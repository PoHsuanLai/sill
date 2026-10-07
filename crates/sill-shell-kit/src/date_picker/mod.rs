//! DatePicker: `NSDatePicker` (design/30 section 2.3): a date and a time in the textual style
//! (typed-in segments and a stepper) or the graphical one (a `MonthGrid` and a time field).
//! It lives with the month grid it draws, so the shell and an app share one calendar.

pub(crate) mod calendar;
pub mod model;
pub mod view;
