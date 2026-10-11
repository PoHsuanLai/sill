//! The screen-sharing picker: what an app may see, asked of the person the way macOS asks it.
//! transom (the portal backend) calls `org.quire.Shell1.PickCaptureSource`
//! (`sill_services::capture_pick`); this sheet lists the screens and windows the app may take,
//! centred on the banners' output over an Overlay layer that takes the keyboard, and answers
//! through `CaptureCommand::Answer`.
//!
//! A segmented control (Screens, Windows) shows only when the app accepts both; the pointer
//! control only when the app accepts more than one way to draw it. The rows are quire's `List`
//! and `Row`: a click picks (a second source is added when the app takes several), the arrow keys
//! move, Return shares, Space picks the row under the keys, Escape cancels. Allow stays disabled
//! until something is picked, so a stray Return shares nothing.
//!
//! Plain icons and names for now; thumbnails wait for casement's capture path. Like the other
//! sheets this is a layer surface on a stock compositor.

mod io;
mod layer;
mod rows;
mod stage;
mod surface;
#[cfg(test)]
mod tests;
mod view;

pub use surface::CaptureSheet;

/// The sheet's stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const CAPTURE_SHEET_CSS: &str = include_str!("style.css");
