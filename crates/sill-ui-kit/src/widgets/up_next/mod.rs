//! The Up Next widget's provider: the next events (`sill_model::events::up_next`
//! over a week ahead, at most three), each a title and a time line, as
//! [`UpNextEntry`]; "No upcoming events" when there are none, and a quiet line naming the source
//! state when the calendars are off or unreadable. A new entry on the clock's minute tick. The
//! widget itself ([`UpNext`], `widget.rs`) is a `sill_shell_kit::prelude::Widget` of kind `sill.up-next`. The
//! same agenda feeds the month widget's events ([`today_events`]), in the same colours.
//!
//! The most events asked for is a ceiling, not a promise. The card's body is a fixed number of pixels
//! (`--widget-cell`, `--widget-gap`; quire's card), so asking for more rows than it holds would
//! draw the last one half off the bottom, clipped by the card's `overflow:hidden`. The view draws
//! at most [`EVENT_ROWS`], measured once against a real render (`ds_harness::Harness`) rather
//! than guessed.

/// This widget's stylesheet, in [`crate::widgets::widgets_css`].
pub const CSS: &str = include_str!("style.css");

mod io;
mod provider;
#[cfg(test)]
mod tests;
mod widget;

pub use io::UpNextProvider;
pub use provider::{
    EVENT_ROWS, EventLines, calendar_hue, event_lines, today_events, up_next_entry,
};
pub use widget::{DayLine, UpNext, UpNextEntry, UpNextFace, UpNextRow};
