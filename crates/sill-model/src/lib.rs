//! The shell's pure vocabulary: units, identifiers, attempts, and every service's state,
//! commands, events and effects with the pure `step` that is the only place its state changes.
//!
//! Nothing here does I/O or reaches a runtime, a bus or a UI (`scripts/check-boundary.sh`):
//! `sill-services` carries the effects, and every transition is table-tested without a bus.

pub mod app_menu;
pub mod apps;
pub mod attempt;
pub mod auth;
pub mod badges;
pub mod battery;
pub mod bluetooth;
pub mod clipboard;
pub mod clock;
pub mod companion;
pub mod cosmic_wl;
pub mod displays;
pub mod events;
pub mod idle;
pub mod ids;
pub mod intents;
pub mod kbd_backlight;
pub mod keyboard;
pub mod keycap;
pub mod launch;
pub mod locker;
pub mod network;
pub mod notifications;
pub mod now_playing;
pub mod pins;
pub mod polkit_agent;
pub mod power_profile;
pub mod results;
pub mod screenshots;
pub mod session;
pub mod tray;
pub mod units;
pub mod volume;
pub mod wallpaper;
pub mod wifi;
