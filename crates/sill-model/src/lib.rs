//! The shell's pure vocabulary: units, identifiers, attempts, and every service's state,
//! commands, events and effects with the pure `step` that is the only place its state changes.
//!
//! Nothing here does I/O or reaches a runtime, a bus or a UI (`scripts/check-boundary.sh`):
//! `sill-services` carries the effects, and every transition is table-tested without a bus.

pub mod attempt;
pub mod auth;
pub mod badges;
pub mod battery;
pub mod clock;
pub mod cosmic_wl;
pub mod idle;
pub mod ids;
pub mod kbd_backlight;
pub mod locker;
pub mod now_playing;
pub mod pins;
pub mod power_profile;
pub mod session;
pub mod tray;
pub mod units;
pub mod volume;
pub mod wallpaper;
