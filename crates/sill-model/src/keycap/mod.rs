//! The keycap daemon as the shell sees it: its connection state, the events that move it, and
//! the pure decisions around it: when the person should be told something and what, which focus
//! and config keycap is sent, when systemd is asked about the unit and how long to wait before
//! connecting again.

mod backoff;
mod command;
mod focus;
mod model;
mod notice;
mod step;
#[cfg(test)]
mod tests;
mod unit;

pub use backoff::reconnect_delay;
pub use command::{KeycapAsk, KeycapCommand, KeycapEvent, UnitState};
pub use focus::{Wants, focus_to_send, user_config};
pub use model::{Attention, KeycapState};
pub use notice::{Notice, NoticeChange, notice_for, notice_text};
pub use step::{attention, step};
pub use unit::{UNIT_GRACE, UNIT_RECHECK, unit_state, watching_unit};
