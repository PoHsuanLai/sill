//! The World Clock widget's provider: each zone in `widgets.world_clocks` as
//! quire's `ClockCity`, the city's time, day or night there, and how far it is from here
//! ("Today, +7h", "Tomorrow, +13h") as its note, a new entry on the clock's minute tick. quire's
//! `WorldClockWidget` draws them (dials on the desktop, digits in the tile, up to
//! `ds_shell::widget::MAX_CITIES`). A zone jiff does not know is left out.

mod io;
mod provider;
#[cfg(test)]
mod tests;

pub use io::WorldClockProvider;
pub use provider::{ClockRow, ZoneName, clock_entry, clock_rows, parse_zones, phase_of};
