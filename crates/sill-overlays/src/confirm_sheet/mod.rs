//! The confirmation sheet (design/32 section 8, phase A): what intentd asks the person before it
//! lets the terminal, the companion or an outside agent do something that asks. One Overlay per
//! output, kept warm and hidden like the polkit prompt, shown on the main display (the banners'
//! output) while the intents service's queue holds a question. It draws the question the router
//! formatted (`sill_model::intents::sheet::SheetView`: the action's own label, who asks, the
//! arguments, the reasons) on quire's `Sheet`, centred, with quire's `FactList` and `Button`s, and
//! answers through `IntentsCommand::Answer`.
//!
//! Arming: the buttons are drawn disabled for [`gate::ARM_MS`] after the sheet appears and Return
//! is ignored, so a key typed for something else cannot answer it; Escape denies at any time. A
//! destructive action asks for a hold ([`gate::HOLD_MS`]) on its Allow button, never a press. The
//! second button of a question offered `OnceOrFromTerminal` is "Allow from the terminal: this
//! action, until logout"; of `OnceOrAlways`, "Always allow".
//!
//! This is phase A: a layer surface on a stock compositor, so it is not unforgeable and its
//! receipt says `SheetFallback`, never `HardwareSeat`. Phase B places the same surface on the
//! compositor's trusted surface.

mod gate;
mod io;
mod layer;
mod surface;
#[cfg(test)]
mod tests;
mod view;

pub use surface::ConfirmSheet;

/// The confirmation sheet's stylesheet (lint-clean under `Profile::Strict`,
/// `crates/sill/tests/lint.rs`).
pub const CONFIRM_SHEET_CSS: &str = include_str!("style.css");
