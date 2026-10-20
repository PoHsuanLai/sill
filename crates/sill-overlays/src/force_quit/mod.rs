//! The Force Quit Applications panel (Option+Command+Escape, `sill force-quit`): a sheet
//! centred on one output listing the running apps with their icons, "(not responding)" beside
//! the ones that do not answer the desktop's pings, and a Force Quit button that asks "Do you want
//! to force quit "App"? You will lose any unsaved changes." before it ends the app's
//! processes. A hung app starts selected. The shell itself is not an app of the desktop's list,
//! so it is not offered; the Mac lists Finder with Relaunch, this desktop has no counterpart.
//!
//! The daemon writes the list (`ForceQuitLink`, from the compositor's `DesktopView`) and the
//! opening; the confirmed app goes back on the link and the daemon sends
//! `DesktopCommand::ForceQuitApp`. Escape closes the panel, or answers the question when it is
//! up; a press away from the sheet closes it. Like the other sheets this is a layer surface on a
//! stock compositor.

mod io;
mod model;
mod step;
mod surface;
#[cfg(test)]
mod tests;
mod view;

pub use surface::ForceQuit;

/// The panel's stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const FORCE_QUIT_CSS: &str = include_str!("style.css");
