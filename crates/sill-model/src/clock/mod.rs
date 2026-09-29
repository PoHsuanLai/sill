//! The bar clock's text, formatted from a civil date-time.

mod command;
mod format;
mod model;

pub use command::ClockCommand;
pub use format::{format_clock, until_next_minute};
pub use model::ClockText;
