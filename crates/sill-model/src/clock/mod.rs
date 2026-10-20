//! The bar clock: the minute the service publishes, and the writing of a time of day.

mod command;
mod format;
mod model;

pub use command::ClockCommand;
pub use format::{format_clock, time_digits, time_of_day, time_of_minute, until_next_minute};
pub use model::{ClockHours, ClockMinute};
