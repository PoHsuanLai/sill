//! Night Shift: warming the screen at night. casement does the warming (its software night light,
//! driven by `[night_light]` in its `settings.toml`); the shell is where the person asks for it
//! ("Turn On Until Tomorrow", the warmth) and sees it, and it keeps the manual turn-on's end
//! beside casement's own schedule. Where sill already warms the displays over DDC/CI, casement
//! is kept off so the two do not double. Pure; `sill-services` reads and writes the files.

mod command;
mod model;
mod shown;
mod step;
#[cfg(test)]
mod tests;

pub use command::{NightShiftCommand, NightShiftEffect, NightShiftEvent, Now};
pub use model::{
    Backend, Manual, MinuteOfDay, NightLight, NightShift, Reach, Schedule, UnixSeconds, Warmth,
};
pub use shown::{Shown, shown, warmth_at, warmth_fraction};
pub use step::{step, until_tomorrow};
