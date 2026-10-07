//! The World Clock widget's provider: each zone in `widgets.world_clocks` as
//! quire's `ClockCity`, the city's time, day or night there, and under it two notes: its day
//! ("Today", or its date, "Sat 27 Sep", when that differs from here's) and how far it is from
//! here ("+7h"), a new entry on the clock's minute tick. quire's
//! `WorldClockWidget` draws them (dials on the desktop, digits in the tile, up to
//! `sill_shell_kit::widget::MAX_CITIES`). A zone jiff does not know is left out.

mod io;
mod provider;
#[cfg(test)]
mod tests;

pub use io::WorldClockProvider;
pub use provider::{ClockRow, DayThere, ZoneName, clock_entry, clock_rows, parse_zones, phase_of};
