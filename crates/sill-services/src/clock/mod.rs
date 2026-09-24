//! The bar clock: a task that sleeps to each minute boundary and publishes the formatted time
//! (plan hooks table, `use_clock`: "sleep to minute boundary; pure format_clock").

#![allow(unused_variables, dead_code)] // Freeze stubs

mod format;

pub use format::{ClockText, format_clock, until_next_minute};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// What the clock service can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockCommand {
    /// Re-read the time zone and reformat now (after a resume or a time-zone change).
    Refresh,
}

/// Start the clock task on `rt`: formats the local time now, then at every minute boundary.
pub fn spawn(rt: &Handle) -> (watch::Receiver<ClockText>, Control<ClockCommand>) {
    todo!("M1 bar-ui: jiff::Zoned::now() -> format_clock, sleep(until_next_minute)")
}
